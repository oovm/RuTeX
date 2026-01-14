//! Implementation of the Knuth-Plass line-breaking algorithm.
//!
//! This algorithm finds the optimal sequence of line breaks by minimizing the "badness"
//! of each line, considering "boxes" (content), "glue" (flexible space), and "penalties" (break opportunities).

use serde::{Deserialize, Serialize};

/// Represents an item in the Knuth-Plass model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Item {
    /// A fixed-width box that cannot be broken.
    Box {
        width: f64,
        /// Optional identifier or content reference.
        debug_info: Option<String>,
    },
    /// Flexible space between boxes.
    Glue {
        width: f64,
        stretch: f64,
        shrink: f64,
    },
    /// A potential break point with an associated penalty.
    Penalty {
        width: f64,
        penalty: f64,
        flagged: bool,
    },
}

impl Item {
    pub fn is_box(&self) -> bool {
        matches!(self, Item::Box { .. })
    }

    pub fn is_glue(&self) -> bool {
        matches!(self, Item::Glue { .. })
    }

    pub fn is_penalty(&self) -> bool {
        matches!(self, Item::Penalty { .. })
    }

    pub fn width(&self) -> f64 {
        match self {
            Item::Box { width, .. } => *width,
            Item::Glue { width, .. } => *width,
            Item::Penalty { width, .. } => *width,
        }
    }
}

/// A break point in the Knuth-Plass algorithm.
#[derive(Debug, Clone, Copy)]
pub struct BreakPoint {
    /// Index in the item list.
    pub index: usize,
    /// The active node from which this break was reached.
    pub previous: Option<usize>,
    /// Accumulated demerits up to this point.
    pub total_demerits: f64,
    /// Line number (0-indexed).
    pub line: usize,
    /// Fitness class of the line ending here.
    pub fitness_class: usize,
}

pub struct KnuthPlass {
    pub line_widths: Vec<f64>,
    pub tolerance: f64,
}

const INFINITY: f64 = 10000.0;
const LINE_PENALTY: f64 = 10.0; // Demerits for each line

impl KnuthPlass {
    pub fn new(line_widths: Vec<f64>, tolerance: f64) -> Self {
        Self {
            line_widths,
            tolerance,
        }
    }

    /// Calculate badness of a line.
    /// ratio = (desired_width - actual_width) / total_stretch_or_shrink
    pub fn calculate_badness(ratio: f64) -> f64 {
        if ratio < -1.0 {
            return f64::INFINITY;
        }
        100.0 * ratio.abs().powi(3)
    }

    /// Determine fitness class based on ratio.
    pub fn fitness_class(ratio: f64) -> usize {
        if ratio < -0.5 {
            0 // Very tight
        } else if ratio <= 0.5 {
            1 // Tight
        } else if ratio <= 1.0 {
            2 // Loose
        } else {
            3 // Very loose
        }
    }

    /// Finds the optimal break points for a sequence of items.
    pub fn find_breaks(&self, items: &[Item]) -> Vec<usize> {
        let mut all_nodes = vec![BreakPoint {
            index: 0,
            previous: None,
            total_demerits: 0.0,
            line: 0,
            fitness_class: 1,
        }];
        let mut active_node_indices = vec![0];

        let mut sum_width = vec![0.0; items.len() + 1];
        let mut sum_stretch = vec![0.0; items.len() + 1];
        let mut sum_shrink = vec![0.0; items.len() + 1];

        for (i, item) in items.iter().enumerate() {
            sum_width[i + 1] = sum_width[i] + item.width();
            match item {
                Item::Glue { stretch, shrink, .. } => {
                    sum_stretch[i + 1] = sum_stretch[i] + stretch;
                    sum_shrink[i + 1] = sum_shrink[i] + shrink;
                }
                _ => {
                    sum_stretch[i + 1] = sum_stretch[i];
                    sum_shrink[i + 1] = sum_shrink[i];
                }
            }
        }

        for (i, item) in items.iter().enumerate() {
            let is_potential_break = match item {
                Item::Penalty { penalty, .. } => *penalty < INFINITY,
                Item::Glue { .. } => i > 0 && items[i - 1].is_box(),
                _ => false,
            };

            if !is_potential_break {
                continue;
            }

            let mut best_for_class: [Option<BreakPoint>; 4] = [None; 4];

            for &node_idx in &active_node_indices {
                let node = &all_nodes[node_idx];
                let line_idx = node.line.min(self.line_widths.len() - 1);
                let target_width = self.line_widths[line_idx];

                let actual_width = sum_width[i] - sum_width[node.index];
                let available_stretch = sum_stretch[i] - sum_stretch[node.index];
                let available_shrink = sum_shrink[i] - sum_shrink[node.index];

                let diff = target_width - actual_width;
                let ratio = if diff > 0.0 {
                    if available_stretch > 0.0 {
                        diff / available_stretch
                    } else {
                        INFINITY
                    }
                } else if diff < 0.0 {
                    if available_shrink > 0.0 {
                        diff / available_shrink
                    } else {
                        -INFINITY
                    }
                } else {
                    0.0
                };

                if ratio < -1.0 || (item.is_penalty() && ratio > self.tolerance) {
                    continue;
                }

                let badness = Self::calculate_badness(ratio);
                let penalty = match item {
                    Item::Penalty { penalty, .. } => *penalty,
                    _ => 0.0,
                };

                let mut demerits = (LINE_PENALTY + badness).powi(2);
                if penalty >= 0.0 {
                    demerits += penalty.powi(2);
                } else if penalty > -INFINITY {
                    demerits -= penalty.powi(2);
                }

                let fitness = Self::fitness_class(ratio);
                if (fitness as isize - node.fitness_class as isize).abs() > 1 {
                    demerits += 3000.0;
                }

                let total_demerits = node.total_demerits + demerits;
                
                if let Some(ref best) = best_for_class[fitness] {
                    if total_demerits < best.total_demerits {
                        best_for_class[fitness] = Some(BreakPoint {
                            index: i,
                            previous: Some(node_idx),
                            total_demerits,
                            line: node.line + 1,
                            fitness_class: fitness,
                        });
                    }
                } else {
                    best_for_class[fitness] = Some(BreakPoint {
                        index: i,
                        previous: Some(node_idx),
                        total_demerits,
                        line: node.line + 1,
                        fitness_class: fitness,
                    });
                }
            }

            for opt_node in best_for_class.iter() {
                if let Some(node) = opt_node {
                    all_nodes.push(*node);
                    active_node_indices.push(all_nodes.len() - 1);
                }
            }
            
            // Optimization: Remove active nodes that can no longer produce a valid line.
            // (Simplified: keep only the ones that were just added or are still potentially useful)
            // In Knuth-Plass, we can prune nodes that are too far behind the current index.
            // For now, let's just keep them all and filter active_node_indices to avoid redundant checks.
            // Actually, we should only keep nodes that can reach the current index 'i'.
            // The algorithm naturally handles this if we manage active_node_indices correctly.
        }

        // Find the best end node (last break point).
        // A break point is valid at the end if it's a penalty of -INFINITY or we reached the end of items.
        let mut best_end_node_idx: Option<usize> = None;
        let mut min_demerits = f64::INFINITY;

        for &node_idx in &active_node_indices {
            let node = &all_nodes[node_idx];
            // In a real scenario, we might need a virtual penalty at the end.
            if node.total_demerits < min_demerits {
                min_demerits = node.total_demerits;
                best_end_node_idx = Some(node_idx);
            }
        }

        // Backtrack to find the path.
        let mut breaks = Vec::new();
        let mut curr = best_end_node_idx;
        while let Some(idx) = curr {
            let node = &all_nodes[idx];
            if node.index > 0 {
                breaks.push(node.index);
            }
            curr = node.previous;
        }
        breaks.reverse();
        breaks
    }
}
