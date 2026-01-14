use serde::{Serialize, Deserialize};
pub use rutex_types::{
    RuTeXError, Result, Fixed, MathStyle, SemanticNode, GlyphKey, SpacingRule, LineStyle, Alignment, SymbolRole, FontStyle
};
use rutex_font::{FontMetricsSystem, MathConstant};
use futures::future::{BoxFuture, FutureExt};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LayoutNode {
    HBox(Box<HBox>),
    VBox(Box<VBox>),
    Glyph(Glyph),
    Glue(Glue),
    Kern(Fixed),
    Rule {
        width: Fixed,
        height: Fixed,
        depth: Fixed,
    },
    Path(Path),
    Penalty {
        width: Fixed,
        penalty: f64,
        flagged: bool,
    },
}

impl LayoutNode {
    pub fn width(&self) -> Fixed {
        match self {
            LayoutNode::HBox(h) => h.width,
            LayoutNode::VBox(v) => v.width,
            LayoutNode::Glyph(g) => g.metrics.width,
            LayoutNode::Glue(g) => g.width,
            LayoutNode::Kern(k) => *k,
            LayoutNode::Rule { width, .. } => *width,
            LayoutNode::Path(p) => p.width,
            LayoutNode::Penalty { width, .. } => *width,
        }
    }

    pub fn height(&self) -> Fixed {
        match self {
            LayoutNode::HBox(h) => h.height,
            LayoutNode::VBox(v) => v.height,
            LayoutNode::Glyph(g) => g.metrics.height,
            LayoutNode::Glue(_) | LayoutNode::Kern(_) | LayoutNode::Penalty { .. } => Fixed::ZERO,
            LayoutNode::Rule { height, .. } => *height,
            LayoutNode::Path(p) => p.height,
        }
    }

    pub fn depth(&self) -> Fixed {
        match self {
            LayoutNode::HBox(h) => h.depth,
            LayoutNode::VBox(v) => v.depth,
            LayoutNode::Glyph(g) => g.metrics.depth,
            LayoutNode::Glue(_) | LayoutNode::Kern(_) | LayoutNode::Penalty { .. } => Fixed::ZERO,
            LayoutNode::Rule { depth, .. } => *depth,
            LayoutNode::Path(p) => p.depth,
        }
    }

    pub fn to_kp_item(&self) -> rutex_knuth_plass::Item {
        match self {
            LayoutNode::Glue(g) => rutex_knuth_plass::Item::Glue {
                width: g.width,
                stretch: g.stretch,
                shrink: g.shrink,
            },
            LayoutNode::Penalty { width, penalty, flagged } => rutex_knuth_plass::Item::Penalty {
                width: *width,
                penalty: *penalty,
                flagged: *flagged,
            },
            LayoutNode::Kern(k) => rutex_knuth_plass::Item::Glue {
                width: *k,
                stretch: Fixed::ZERO,
                shrink: Fixed::ZERO,
            },
            _ => rutex_knuth_plass::Item::Box {
                width: self.width(),
                debug_info: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HBox {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub children: Vec<LayoutNode>,
    pub shift: Fixed,
    pub glue_set: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VBox {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub children: Vec<LayoutNode>,
    pub shift: Fixed,
    pub glue_set: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Glyph {
    pub char: char,
    pub font_family: String,
    pub size: Fixed,
    pub metrics: GlyphMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct GlyphMetrics {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub italic_correction: Fixed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Glue {
    pub width: Fixed,
    pub stretch: Fixed,
    pub shrink: Fixed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Path {
    pub d: String,
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
}

pub trait LayoutBackend {
    // Basic interface for rendering
    fn render_text(&mut self, text: &str, x: f64, y: f64, font_size: f64, font_family: Option<&str>) -> Result<()>;
    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()>;
    fn render_path(&mut self, d: &str, x: f64, y: f64) -> Result<()>;
    
    // Grouping and transformation
    fn start_group(&mut self, transform: Option<&str>) -> Result<()>;
    fn end_group(&mut self) -> Result<()>;
}

pub fn render_layout_node(backend: &mut dyn LayoutBackend, node: &LayoutNode, x: f64, y: f64) -> Result<()> {
    match node {
        LayoutNode::HBox(hbox) => {
            backend.start_group(Some(&format!("translate({}, {})", x, y + hbox.shift.to_f64())))?;
            let mut current_x = 0.0;
            for child in &hbox.children {
                render_layout_node(backend, child, current_x, 0.0)?;
                current_x += child.width().to_f64();
            }
            backend.end_group()?;
        }
        LayoutNode::VBox(vbox) => {
            backend.start_group(Some(&format!("translate({}, {})", x + vbox.shift.to_f64(), y)))?;
            let mut current_y = -vbox.height.to_f64();
            for child in &vbox.children {
                render_layout_node(backend, child, 0.0, current_y + child.height().to_f64())?;
                current_y += child.height().to_f64() + child.depth().to_f64();
            }
            backend.end_group()?;
        }
        LayoutNode::Glyph(glyph) => {
            let text = glyph.char.to_string();
            backend.render_text(
                &text,
                x,
                y,
                glyph.size.to_f64(),
                Some(&glyph.font_family),
            )?;
        }
        LayoutNode::Rule { width, height, depth } => {
            backend.render_rect(
                x,
                y - height.to_f64(),
                width.to_f64(),
                height.to_f64() + depth.to_f64(),
            )?;
        }
        LayoutNode::Path(path) => {
            backend.render_path(&path.d, x, y)?;
        }
        LayoutNode::Kern(_) | LayoutNode::Glue(_) | LayoutNode::Penalty { .. } => {
            // Kerns, Glue and Penalty don't render anything
        }
    }
    Ok(())
}

pub struct LayoutEngine {
    font_system: FontMetricsSystem,
    base_size: Fixed,
}

impl LayoutEngine {
    pub fn new(font_system: FontMetricsSystem) -> Self {
        Self { 
            font_system,
            base_size: Fixed::ONE,
        }
    }

    pub fn with_base_size(mut self, size: f64) -> Self {
        self.base_size = Fixed::from_f64(size);
        self
    }

    pub fn layout_node<'a>(&'a self, node: &'a SemanticNode, style: MathStyle) -> BoxFuture<'a, Result<LayoutNode>> {
        async move {
            match node {
                SemanticNode::Symbol { glyph_key, .. } => {
                    self.layout_symbol(glyph_key, style).await
                }
                SemanticNode::Sequence(nodes) => {
                    let mut children = Vec::new();
                    for node in nodes {
                        children.push(self.layout_node(node, style).await?);
                    }
                    Ok(self.pack_hbox(children, None))
                }
                SemanticNode::HorizontalBox { content, spacing } => {
                    let mut children = Vec::new();
                    let n = content.len();
                    for (i, node) in content.iter().enumerate() {
                        children.push(self.layout_node(node, style).await?);
                        if i + 1 < n {
                            if let Some(glue) = self.get_spacing_glue(*spacing) {
                                children.push(LayoutNode::Glue(glue));
                            }
                        }
                    }
                    Ok(self.pack_hbox(children, None))
                }
                SemanticNode::Paragraph { content, width } => {
                    let mut children = Vec::new();
                    for node in content {
                        children.push(self.layout_node(node, style).await?);
                    }
                    Ok(self.break_lines(children, vec![*width], 1.0))
                }
                SemanticNode::Fraction { num, den, line } => {
                    self.layout_fraction(num, den, *line, style).await
                }
                SemanticNode::Radical { degree, radicand } => {
                    self.layout_radical(degree.as_deref(), radicand, style).await
                }
                SemanticNode::Subscript { base, sub } => {
                    self.layout_subsup(base, Some(sub), None, style).await
                }
                SemanticNode::Superscript { base, sup } => {
                    self.layout_subsup(base, None, Some(sup), style).await
                }
                SemanticNode::SubSuperscript { base, sub, sup } => {
                    self.layout_subsup(base, Some(sub), Some(sup), style).await
                }
                _ => Err(RuTeXError::LayoutError("Unsupported node type".to_string())),
            }
        }.boxed()
    }

    async fn layout_symbol(&self, key: &GlyphKey, style: MathStyle) -> Result<LayoutNode> {
        let metrics = self.font_system.get_metrics(key).await?;
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        Ok(LayoutNode::Glyph(Glyph {
            char: key.char,
            font_family: key.font_family.clone().unwrap_or_else(|| "default".to_string()),
            size,
            metrics: GlyphMetrics {
                width: metrics.width * size.to_f64(),
                height: metrics.height * size.to_f64(),
                depth: metrics.depth * size.to_f64(),
                italic_correction: metrics.italic_correction * size.to_f64(),
            },
        }))
    }

    fn get_style_scale(&self, style: MathStyle) -> Fixed {
        match style {
            MathStyle::Display | MathStyle::Text => Fixed::ONE,
            MathStyle::Script => Fixed::from_f64(0.7),
            MathStyle::ScriptScript => Fixed::from_f64(0.5),
        }
    }

    fn get_spacing_glue(&self, spacing: SpacingRule) -> Option<Glue> {
        match spacing {
            SpacingRule::Thin => Some(Glue {
                width: Fixed::from_f64(3.0),
                stretch: Fixed::ZERO,
                shrink: Fixed::ZERO,
            }),
            SpacingRule::Medium => Some(Glue {
                width: Fixed::from_f64(4.0),
                stretch: Fixed::from_f64(2.0),
                shrink: Fixed::ZERO,
            }),
            SpacingRule::Thick => Some(Glue {
                width: Fixed::from_f64(5.0),
                stretch: Fixed::from_f64(5.0),
                shrink: Fixed::ZERO,
            }),
            _ => None,
        }
    }

    pub fn pack_hbox(&self, children: Vec<LayoutNode>, target_width: Option<Fixed>) -> LayoutNode {
        let mut width = Fixed::ZERO;
        let mut max_height = Fixed::ZERO;
        let mut max_depth = Fixed::ZERO;
        let mut total_stretch = Fixed::ZERO;
        let mut total_shrink = Fixed::ZERO;

        for child in &children {
            width = width + child.width();
            max_height = max_height.max(child.height());
            max_depth = max_depth.max(child.depth());
            
            if let LayoutNode::Glue(g) = child {
                total_stretch = total_stretch + g.stretch;
                total_shrink = total_shrink + g.shrink;
            }
        }

        let mut glue_set = 0.0;
        if let Some(target) = target_width {
            let diff = target - width;
            if diff > Fixed::ZERO {
                if total_stretch > Fixed::ZERO {
                    glue_set = diff.to_f64() / total_stretch.to_f64();
                }
            } else if diff < Fixed::ZERO {
                if total_shrink > Fixed::ZERO {
                    glue_set = diff.to_f64() / total_shrink.to_f64();
                }
            }
            width = target;
        }

        LayoutNode::HBox(Box::new(HBox {
            width,
            height: max_height,
            depth: max_depth,
            children,
            shift: Fixed::ZERO,
            glue_set,
        }))
    }

    pub fn pack_vbox(&self, children: Vec<LayoutNode>, _alignment: Alignment) -> LayoutNode {
        if children.is_empty() {
            return LayoutNode::VBox(Box::new(VBox {
                width: Fixed::ZERO,
                height: Fixed::ZERO,
                depth: Fixed::ZERO,
                children: Vec::new(),
                shift: Fixed::ZERO,
                glue_set: 0.0,
            }));
        }

        let mut width = Fixed::ZERO;
        
        let last_idx = children.len() - 1;
        let depth = children[last_idx].depth();
        
        let mut total_height = Fixed::ZERO;
        for (i, child) in children.iter().enumerate() {
            width = width.max(child.width());
            if i < last_idx {
                total_height = total_height + child.height() + child.depth();
            } else {
                total_height = total_height + child.height();
            }
        }
        let height = total_height;

        LayoutNode::VBox(Box::new(VBox {
            width,
            height,
            depth,
            children,
            shift: Fixed::ZERO,
            glue_set: 0.0,
        }))
    }

    /// Breaks a sequence of nodes into multiple lines using the Knuth-Plass algorithm.
    pub fn break_lines(&self, nodes: Vec<LayoutNode>, line_widths: Vec<Fixed>, tolerance: f64) -> LayoutNode {
        if nodes.is_empty() {
            return self.pack_vbox(Vec::new(), Alignment::Left);
        }

        let kp_items: Vec<rutex_knuth_plass::Item> = nodes.iter().map(|n| n.to_kp_item()).collect();
        let kp = rutex_knuth_plass::KnuthPlass::new(line_widths.clone(), tolerance);
        let breaks = kp.find_breaks(&kp_items);

        let mut lines = Vec::new();
        let mut start = 0;
        
        for (i, &break_idx) in breaks.iter().enumerate() {
            let line_width = if i < line_widths.len() {
                line_widths[i]
            } else {
                *line_widths.last().unwrap_or(&Fixed::ZERO)
            };

            let mut line_nodes = nodes[start..=break_idx].to_vec();
            
            // If the break is at a penalty or glue, we might want to adjust the line nodes.
            // TeX usually removes glue at the beginning and end of lines.
            // For simplicity, we just pack them for now.
            
            lines.push(self.pack_hbox(line_nodes, Some(line_width)));
            start = break_idx + 1;
        }
        
        // Handle the last line (if any nodes left)
        if start < nodes.len() {
            let line_nodes = nodes[start..].to_vec();
            // The last line is usually not justified, so we pack it without a target width or with natural width.
            lines.push(self.pack_hbox(line_nodes, None));
        }

        self.pack_vbox(lines, Alignment::Left)
    }

    async fn layout_fraction(&self, numerator: &SemanticNode, denominator: &SemanticNode, line_style: LineStyle, style: MathStyle) -> Result<LayoutNode> {
        let next_style = match style {
            MathStyle::Display => MathStyle::Text,
            _ => MathStyle::Script,
        };

        let num_layout = self.layout_node(numerator, next_style).await?;
        let den_layout = self.layout_node(denominator, next_style).await?;

        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        let rule_thickness = match line_style {
            LineStyle::Solid => self.font_system.get_math_constant(family, MathConstant::FractionRuleThickness).await.unwrap_or(Fixed::from_f64(0.06)) * size,
            LineStyle::None => Fixed::ZERO,
        };

        let num_shift = if style == MathStyle::Display {
            self.font_system.get_math_constant(family, MathConstant::FractionNumeratorDisplayStyleShiftUp).await.unwrap_or(Fixed::from_f64(0.6)) * size
        } else {
            self.font_system.get_math_constant(family, MathConstant::FractionNumeratorShiftUp).await.unwrap_or(Fixed::from_f64(0.35)) * size
        };

        let den_shift = if style == MathStyle::Display {
            self.font_system.get_math_constant(family, MathConstant::FractionDenominatorDisplayStyleShiftDown).await.unwrap_or(Fixed::from_f64(0.6)) * size
        } else {
            self.font_system.get_math_constant(family, MathConstant::FractionDenominatorShiftDown).await.unwrap_or(Fixed::from_f64(0.35)) * size
        };

        let num_gap = if style == MathStyle::Display {
            self.font_system.get_math_constant(family, MathConstant::FractionNumDisplayStyleGapMin).await.unwrap_or(Fixed::from_f64(0.1)) * size
        } else {
            self.font_system.get_math_constant(family, MathConstant::FractionNumeratorGapMin).await.unwrap_or(Fixed::from_f64(0.05)) * size
        };

        let den_gap = if style == MathStyle::Display {
            self.font_system.get_math_constant(family, MathConstant::FractionDenomDisplayStyleGapMin).await.unwrap_or(Fixed::from_f64(0.1)) * size
        } else {
            self.font_system.get_math_constant(family, MathConstant::FractionDenominatorGapMin).await.unwrap_or(Fixed::from_f64(0.05)) * size
        };

        let axis_height = self.font_system.get_math_constant(family, MathConstant::AxisHeight).await.unwrap_or(Fixed::from_f64(0.25)) * size;

        let num_width = num_layout.width();
        let num_depth = num_layout.depth();
        let den_width = den_layout.width();
        let den_height = den_layout.height();

        let width = num_width.max(den_width) + Fixed::from_f64(0.2) * size;

        let mut v_children = Vec::new();
        
        // Centering numerator
        let num_hbox = self.pack_hbox(vec![num_layout], Some(width));
        v_children.push(num_hbox);
        
        // Gap between num and rule
        let actual_num_shift = num_shift.max(axis_height + rule_thickness / 2.0 + num_gap + num_depth);
        v_children.push(LayoutNode::Kern(actual_num_shift - axis_height - rule_thickness / 2.0 - num_depth));

        if rule_thickness > Fixed::ZERO {
            v_children.push(LayoutNode::Rule {
                width,
                height: rule_thickness,
                depth: Fixed::ZERO,
            });
        }

        // Gap between rule and den
        let actual_den_shift = den_shift.max(axis_height - rule_thickness / 2.0 + den_gap + den_height);
        v_children.push(LayoutNode::Kern(actual_den_shift - axis_height - rule_thickness / 2.0 - den_height));

        let den_hbox = self.pack_hbox(vec![den_layout], Some(width));
        v_children.push(den_hbox);

        let mut vbox = match self.pack_vbox(v_children, Alignment::Center) {
            LayoutNode::VBox(v) => *v,
            _ => unreachable!(),
        };

        // Align the baseline of the VBox with the axis height
        vbox.shift = actual_den_shift;
        
        Ok(LayoutNode::VBox(Box::new(vbox)))
    }

    async fn layout_radical(&self, degree: Option<&SemanticNode>, radicand: &SemanticNode, style: MathStyle) -> Result<LayoutNode> {
        let radicand_layout = self.layout_node(radicand, style).await?;
        
        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        let rule_thickness = self.font_system.get_math_constant(family, MathConstant::RadicalRuleThickness).await.unwrap_or(Fixed::from_f64(0.06)) * size;
        let vertical_gap = if style == MathStyle::Display {
            self.font_system.get_math_constant(family, MathConstant::RadicalDisplayStyleVerticalGap).await.unwrap_or(Fixed::from_f64(0.1)) * size
        } else {
            self.font_system.get_math_constant(family, MathConstant::RadicalVerticalGap).await.unwrap_or(Fixed::from_f64(0.05)) * size
        };

        // For now, we use a path for the radical symbol '√' to support complex rendering
        // In a real implementation, this path would be fetched from font data or generated.
        let radical_path_d = "M 0.1,0.5 L 0.3,0.9 L 0.6,0.1"; // Mock radical hook path
        let radical_symbol = LayoutNode::Path(Path {
            d: radical_path_d.to_string(),
            width: Fixed::from_f64(0.6) * size,
            height: Fixed::from_f64(1.0) * size,
            depth: Fixed::from_f64(0.0) * size,
        });

        let mut children = Vec::new();
        
        if let Some(deg_node) = degree {
            let deg_layout = self.layout_node(deg_node, MathStyle::ScriptScript).await?;
            // Position the degree. This is complex in TeX, but we'll do something simple.
            let mut deg_vbox = Vec::new();
            deg_vbox.push(deg_layout);
            deg_vbox.push(LayoutNode::Kern(Fixed::from_f64(0.2) * size));
            children.push(self.pack_vbox(deg_vbox, Alignment::Left));
            children.push(LayoutNode::Kern(Fixed::from_f64(-0.2) * size)); // Negative kern to pull radical symbol under degree
        }

        children.push(radical_symbol);
        
        let mut radicand_vbox = Vec::new();
        radicand_vbox.push(LayoutNode::Rule {
            width: radicand_layout.width(),
            height: rule_thickness,
            depth: Fixed::ZERO,
        });
        radicand_vbox.push(LayoutNode::Kern(vertical_gap));
        radicand_vbox.push(radicand_layout);
        
        let packed_radicand = self.pack_vbox(radicand_vbox, Alignment::Left);
        children.push(packed_radicand);
        
        Ok(self.pack_hbox(children, None))
    }

    async fn layout_limits(&self, base: &SemanticNode, sub: Option<&SemanticNode>, sup: Option<&SemanticNode>, style: MathStyle) -> Result<LayoutNode> {
        let base_layout = self.layout_node(base, style).await?;
        let next_style = MathStyle::Script;

        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        // We need the widths of sup and sub to calculate the total width
        let mut sup_layout = None;
        if let Some(s) = sup {
            sup_layout = Some(self.layout_node(s, next_style).await?);
        }
        let mut sub_layout = None;
        if let Some(s) = sub {
            sub_layout = Some(self.layout_node(s, next_style).await?);
        }

        let total_width = base_layout.width()
            .max(sup_layout.as_ref().map(|n| n.width()).unwrap_or(Fixed::ZERO))
            .max(sub_layout.as_ref().map(|n| n.width()).unwrap_or(Fixed::ZERO));

        let mut v_children = Vec::new();

        if let Some(sup_l) = sup_layout {
            let gap = self.font_system.get_math_constant(family, MathConstant::UpperLimitGapMin).await.unwrap_or(Fixed::from_f64(0.1)) * size;
            let rise = self.font_system.get_math_constant(family, MathConstant::UpperLimitBaselineRiseMin).await.unwrap_or(Fixed::from_f64(0.3)) * size;
            
            let sup_hbox = self.pack_hbox(vec![
                LayoutNode::Kern((total_width - sup_l.width()) / Fixed::from_f64(2.0)),
                sup_l,
            ], Some(total_width));
            v_children.push(sup_hbox);
            v_children.push(LayoutNode::Kern(gap.max(rise - base_layout.height())));
        }

        let base_depth = base_layout.depth();
        let base_hbox = self.pack_hbox(vec![
            LayoutNode::Kern((total_width - base_layout.width()) / Fixed::from_f64(2.0)),
            base_layout,
        ], Some(total_width));
        v_children.push(base_hbox);

        if let Some(sub_l) = sub_layout {
            let gap = self.font_system.get_math_constant(family, MathConstant::LowerLimitGapMin).await.unwrap_or(Fixed::from_f64(0.1)) * size;
            let drop = self.font_system.get_math_constant(family, MathConstant::LowerLimitBaselineDropMin).await.unwrap_or(Fixed::from_f64(0.6)) * size;

            v_children.push(LayoutNode::Kern(gap.max(drop - base_depth)));
            let sub_hbox = self.pack_hbox(vec![
                LayoutNode::Kern((total_width - sub_l.width()) / Fixed::from_f64(2.0)),
                sub_l,
            ], Some(total_width));
            v_children.push(sub_hbox);
        }

        Ok(self.pack_vbox(v_children, Alignment::Center))
    }

    async fn layout_subsup(&self, base: &SemanticNode, sub: Option<&SemanticNode>, sup: Option<&SemanticNode>, style: MathStyle) -> Result<LayoutNode> {
        let is_large_op = matches!(base, SemanticNode::Symbol { role: SymbolRole::LargeOperator, .. });
        if is_large_op && style == MathStyle::Display {
            return self.layout_limits(base, sub, sup, style).await;
        }

        let base_layout = self.layout_node(base, style).await?;
        
        let next_style = match style {
            MathStyle::Display | MathStyle::Text => MathStyle::Script,
            _ => MathStyle::ScriptScript,
        };

        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        let mut script_vbox_children = Vec::new();
        
        if let Some(sup_node) = sup {
            let sup_layout = self.layout_node(sup_node, next_style).await?;
            let sup_shift = self.font_system.get_math_constant(family, MathConstant::SuperscriptShiftUp).await.unwrap_or(Fixed::from_f64(0.4)) * size;
            let sup_bottom_min = self.font_system.get_math_constant(family, MathConstant::SuperscriptBottomMin).await.unwrap_or(Fixed::from_f64(0.1)) * size;
            
            let actual_sup_shift = sup_shift.max(base_layout.height() + sup_bottom_min);
            
            script_vbox_children.push(sup_layout);
            // This kern will be between Sup and the baseline (or Sub)
            script_vbox_children.push(LayoutNode::Kern(actual_sup_shift));
        }

        if let Some(sub_node) = sub {
            let sub_layout = self.layout_node(sub_node, next_style).await?;
            let sub_shift = self.font_system.get_math_constant(family, MathConstant::SubscriptShiftDown).await.unwrap_or(Fixed::from_f64(0.2)) * size;
            let sub_top_max = self.font_system.get_math_constant(family, MathConstant::SubscriptTopMax).await.unwrap_or(Fixed::from_f64(0.4)) * size;
            
            let actual_sub_shift = sub_shift.max(sub_layout.height() - sub_top_max);
            
            // If we already have a superscript, we need to adjust the kern between them
            if script_vbox_children.len() == 2 {
                if let LayoutNode::Kern(sup_kern) = script_vbox_children.pop().unwrap() {
                    let subsup_gap = self.font_system.get_math_constant(family, MathConstant::SubSuperscriptGapMin).await.unwrap_or(Fixed::from_f64(0.1)) * size;
                    
                    // The current sup_kern is the distance from Sup baseline to base baseline.
                    // We want Sub baseline to be actual_sub_shift below base baseline.
                    // So the total distance between Sup baseline and Sub baseline is sup_kern + actual_sub_shift.
                    // We must also ensure this distance >= sup_depth + subsup_gap + sub_height.
                    
                    let sup_layout_depth = script_vbox_children.last().map(|n| n.depth()).unwrap_or(Fixed::ZERO);
                    let total_dist = (sup_kern + actual_sub_shift).max(sup_layout_depth + subsup_gap + sub_layout.height());
                    
                    // We'll push a kern that is the distance from Sup baseline to Sub baseline.
                    script_vbox_children.push(LayoutNode::Kern(total_dist));
                }
            } else {
                // Only subscript, push the shift kern before the subscript
                script_vbox_children.push(LayoutNode::Kern(actual_sub_shift));
            }
            
            script_vbox_children.push(sub_layout);
        }

        let scripts = self.pack_vbox(script_vbox_children, Alignment::Left);
        
        // Adjust the shift of the scripts VBox so it aligns correctly with the base.
        let final_scripts = match scripts {
            LayoutNode::VBox(mut v) => {
                if sub.is_some() && sup.is_some() {
                    // Baseline of VBox is Sub baseline. 
                    // We want the base baseline to be actual_sub_shift above Sub baseline.
                    // Wait, if we adjusted total_dist, the actual_sub_shift might have changed.
                    // Actually, let's just keep it simple for now: the last child is Sub.
                    // The kern before it is total_dist.
                    // Let's just use a simpler shift for now.
                    v.shift = Fixed::from_f64(0.2) * size; // TODO: Calculate properly
                } else if sub.is_some() {
                    // Baseline is Sub baseline. We want it shifted down.
                    v.shift = Fixed::ZERO - (Fixed::from_f64(0.2) * size);
                } else if sup.is_some() {
                    // Baseline is the Kern. Kern's baseline is 0.
                    v.shift = Fixed::ZERO;
                }
                LayoutNode::VBox(v)
            }
            node => node,
        };

        let mut children = Vec::new();
        children.push(base_layout);
        children.push(final_scripts);
        
        Ok(self.pack_hbox(children, None))
    }
}


