use serde::{Serialize, Deserialize};
pub use rutex_types::{RuTeXError, Result, Fixed, MathStyle, SemanticNode, GlyphKey, SpacingRule, LineStyle, Alignment};
use rutex_font::FontMetricsSystem;

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
        }
    }

    pub fn height(&self) -> Fixed {
        match self {
            LayoutNode::HBox(h) => h.height,
            LayoutNode::VBox(v) => v.height,
            LayoutNode::Glyph(g) => g.metrics.height,
            LayoutNode::Glue(_) | LayoutNode::Kern(_) => Fixed::ZERO,
            LayoutNode::Rule { height, .. } => *height,
        }
    }

    pub fn depth(&self) -> Fixed {
        match self {
            LayoutNode::HBox(h) => h.depth,
            LayoutNode::VBox(v) => v.depth,
            LayoutNode::Glyph(g) => g.metrics.depth,
            LayoutNode::Glue(_) | LayoutNode::Kern(_) => Fixed::ZERO,
            LayoutNode::Rule { depth, .. } => *depth,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HBox {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub shift: Fixed,
    pub children: Vec<LayoutNode>,
    pub glue_set: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VBox {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub shift: Fixed,
    pub children: Vec<LayoutNode>,
    pub glue_set: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Glyph {
    pub char: char,
    pub font_family: String,
    pub size: Fixed,
    pub metrics: GlyphMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
pub enum LayoutItem {
    Box(LayoutNode),
    Glue(Glue),
    Penalty {
        cost: f64,
        width: Fixed,
        flagged: bool,
    },
}

pub struct LayoutEngine<'a> {
    pub font_system: &'a FontMetricsSystem,
}

impl<'a> LayoutEngine<'a> {
    pub fn new(font_system: &'a FontMetricsSystem) -> Self {
        Self { font_system }
    }

    pub async fn layout_node(&self, node: &SemanticNode, style: MathStyle) -> Result<LayoutNode> {
        match node {
            SemanticNode::Symbol { glyph_key, .. } => {
                self.layout_symbol(glyph_key, style).await
            }
            SemanticNode::Sequence(nodes) => {
                let mut children = Vec::new();
                for n in nodes {
                    children.push(self.layout_node(n, style).await?);
                }
                Ok(self.pack_hbox(children))
            }
            SemanticNode::HorizontalBox { content, spacing } => {
                let mut children = Vec::new();
                for (i, n) in content.iter().enumerate() {
                    if i > 0 {
                        if let Some(glue) = self.get_spacing_glue(*spacing) {
                            children.push(LayoutNode::Glue(glue));
                        }
                    }
                    children.push(self.layout_node(n, style).await?);
                }
                Ok(self.pack_hbox(children))
            }
            SemanticNode::VerticalBox { content, alignment } => {
                let mut children = Vec::new();
                for n in content {
                    children.push(self.layout_node(n, style).await?);
                }
                Ok(self.pack_vbox(children, *alignment))
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
            SemanticNode::Text(text) => {
                let mut children = Vec::new();
                for c in text.chars() {
                    let key = GlyphKey {
                        char: c,
                        font_family: None,
                        style: rutex_types::FontStyle::Normal,
                    };
                    children.push(self.layout_symbol(&key, style).await?);
                }
                Ok(self.pack_hbox(children))
            }
        }
    }

    async fn layout_symbol(&self, key: &GlyphKey, style: MathStyle) -> Result<LayoutNode> {
        let scale = self.get_style_scale(style);
        let metrics_font = self.font_system.get_metrics(key).await?;
        
        let metrics = GlyphMetrics {
            width: metrics_font.width * scale.to_f64(),
            height: metrics_font.height * scale.to_f64(),
            depth: metrics_font.depth * scale.to_f64(),
            italic_correction: metrics_font.italic_correction * scale.to_f64(),
        };

        Ok(LayoutNode::Glyph(Glyph {
            char: key.char,
            font_family: key.font_family.clone().unwrap_or_else(|| "default".to_string()),
            size: scale * 10.0,
            metrics,
        }))
    }

    pub fn pack_hbox(&self, children: Vec<LayoutNode>) -> LayoutNode {
        let mut width = Fixed::ZERO;
        let mut height = Fixed::ZERO;
        let mut depth = Fixed::ZERO;

        for child in &children {
            width = width + child.width();
            height = height.max(child.height());
            depth = depth.max(child.depth());
        }

        LayoutNode::HBox(Box::new(HBox {
            width,
            height,
            depth,
            shift: Fixed::ZERO,
            children,
            glue_set: 0.0,
        }))
    }

    pub fn pack_vbox(&self, children: Vec<LayoutNode>, _alignment: Alignment) -> LayoutNode {
        let mut width = Fixed::ZERO;
        let mut height = Fixed::ZERO;
        let mut depth = Fixed::ZERO;

        for (i, child) in children.iter().enumerate() {
            width = width.max(child.width());
            if i == 0 {
                height = child.height();
                depth = child.depth();
            } else {
                depth = depth + child.height() + child.depth();
            }
        }

        LayoutNode::VBox(Box::new(VBox {
            width,
            height,
            depth,
            shift: Fixed::ZERO,
            children,
            glue_set: 0.0,
        }))
    }

    fn get_spacing_glue(&self, rule: SpacingRule) -> Option<Glue> {
        match rule {
            SpacingRule::None => None,
            SpacingRule::Thin => Some(Glue {
                width: Fixed::from_f64(3.0),
                stretch: Fixed::ZERO,
                shrink: Fixed::ZERO,
            }),
            SpacingRule::Medium => Some(Glue {
                width: Fixed::from_f64(4.0),
                stretch: Fixed::from_f64(2.0),
                shrink: Fixed::from_f64(1.0),
            }),
            SpacingRule::Thick => Some(Glue {
                width: Fixed::from_f64(5.0),
                stretch: Fixed::from_f64(5.0),
                shrink: Fixed::ZERO,
            }),
            SpacingRule::Auto => Some(Glue {
                width: Fixed::from_f64(3.0),
                stretch: Fixed::from_f64(3.0),
                shrink: Fixed::from_f64(1.0),
            }),
        }
    }

    async fn layout_fraction(&self, num: &SemanticNode, den: &SemanticNode, line: LineStyle, style: MathStyle) -> Result<LayoutNode> {
        let next_style = match style {
            MathStyle::Display => MathStyle::Text,
            MathStyle::Text => MathStyle::Script,
            MathStyle::Script | MathStyle::ScriptScript => MathStyle::ScriptScript,
        };

        let num_node = self.layout_node(num, next_style).await?;
        let den_node = self.layout_node(den, next_style).await?;

        let width = num_node.width().max(den_node.width());
        
        let mut children = Vec::new();
        children.push(num_node);
        
        if matches!(line, LineStyle::Solid) {
            children.push(LayoutNode::Rule {
                width,
                height: Fixed::from_f64(0.5),
                depth: Fixed::ZERO,
            });
        } else {
            children.push(LayoutNode::Kern(Fixed::from_f64(2.0)));
        }
        
        children.push(den_node);

        Ok(self.pack_vbox(children, Alignment::Center))
    }

    async fn layout_radical(&self, _degree: Option<&SemanticNode>, radicand: &SemanticNode, style: MathStyle) -> Result<LayoutNode> {
        let radicand_layout = self.layout_node(radicand, style).await?;
        
        let radical_key = GlyphKey {
            char: '√',
            font_family: None,
            style: rutex_types::FontStyle::Normal,
        };
        let radical_sym = self.layout_symbol(&radical_key, style).await?;
        
        let mut children = Vec::new();
        children.push(radical_sym);
        
        let overline = LayoutNode::Rule {
            width: radicand_layout.width(),
            height: Fixed::from_f64(0.5),
            depth: Fixed::ZERO,
        };
        
        let vbox_content = vec![overline, radicand_layout];
        let radicand_with_bar = self.pack_vbox(vbox_content, Alignment::Left);
        
        children.push(radicand_with_bar);
        
        Ok(self.pack_hbox(children))
    }

    async fn layout_subsup(&self, base: &SemanticNode, sub: Option<&SemanticNode>, sup: Option<&SemanticNode>, style: MathStyle) -> Result<LayoutNode> {
        let base_layout = self.layout_node(base, style).await?;
        
        let next_style = match style {
            MathStyle::Display | MathStyle::Text => MathStyle::Script,
            _ => MathStyle::ScriptScript,
        };

        let mut children = vec![base_layout];
        
        if let Some(sup_node) = sup {
            let sup_layout = self.layout_node(sup_node, next_style).await?;
            let v = VBox {
                width: sup_layout.width(),
                height: sup_layout.height(),
                depth: sup_layout.depth(),
                shift: Fixed::from_f64(-8.0),
                children: vec![sup_layout],
                glue_set: 0.0,
            };
            children.push(LayoutNode::VBox(Box::new(v)));
        }

        if let Some(sub_node) = sub {
            let sub_layout = self.layout_node(sub_node, next_style).await?;
            let v = VBox {
                width: sub_layout.width(),
                height: sub_layout.height(),
                depth: sub_layout.depth(),
                shift: Fixed::from_f64(4.0),
                children: vec![sub_layout],
                glue_set: 0.0,
            };
            children.push(LayoutNode::VBox(Box::new(v)));
        }

        Ok(self.pack_hbox(children))
    }

    fn get_style_scale(&self, style: MathStyle) -> Fixed {
        match style {
            MathStyle::Display | MathStyle::Text => Fixed::ONE,
            MathStyle::Script => Fixed::from_f64(0.7),
            MathStyle::ScriptScript => Fixed::from_f64(0.5),
        }
    }
}

pub fn knuth_plass_line_break(_items: &[LayoutItem], _line_widths: &[Fixed]) -> Vec<usize> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use rutex_types::{GlyphKey, FontStyle, SymbolRole};
    use rutex_font::{FontLoader, FontMetricsSystem, GlyphMetrics as FontGlyphMetrics};
    use async_trait::async_trait;

    struct MockLoader;
    #[async_trait]
    impl FontLoader for MockLoader {
        async fn load_font_data(&self, _family: &str) -> Result<Arc<Vec<u8>>> {
            Err(RuTeXError::FontError { glyph: "".to_string(), message: "Mock loader".to_string() })
        }
    }

    #[tokio::test]
    async fn test_basic_layout() {
        let loader = Arc::new(MockLoader);
        let font_system = FontMetricsSystem::new(loader);
        
        let key = GlyphKey { char: 'a', font_family: None, style: FontStyle::Normal };
        font_system.insert_metrics(key.clone(), FontGlyphMetrics {
            width: Fixed::from_f64(10.0),
            height: Fixed::from_f64(8.0),
            depth: Fixed::from_f64(2.0),
            italic_correction: Fixed::ZERO,
        });

        let engine = LayoutEngine::new(&font_system);
        let node = SemanticNode::Symbol { glyph_key: key, role: SymbolRole::Ordinary };
        
        let layout = engine.layout_node(&node, MathStyle::Text).await.unwrap();
        
        assert_eq!(layout.width(), Fixed::from_f64(10.0));
        assert_eq!(layout.height(), Fixed::from_f64(8.0));
    }

    #[tokio::test]
    async fn test_sequence_layout() {
        let loader = Arc::new(MockLoader);
        let font_system = FontMetricsSystem::new(loader);
        
        let key_a = GlyphKey { char: 'a', font_family: None, style: FontStyle::Normal };
        let key_b = GlyphKey { char: 'b', font_family: None, style: FontStyle::Normal };
        
        font_system.insert_metrics(key_a.clone(), FontGlyphMetrics {
            width: Fixed::from_f64(10.0),
            height: Fixed::from_f64(8.0),
            depth: Fixed::from_f64(2.0),
            italic_correction: Fixed::ZERO,
        });
        font_system.insert_metrics(key_b.clone(), FontGlyphMetrics {
            width: Fixed::from_f64(12.0),
            height: Fixed::from_f64(9.0),
            depth: Fixed::from_f64(1.0),
            italic_correction: Fixed::ZERO,
        });

        let engine = LayoutEngine::new(&font_system);
        let node = SemanticNode::Sequence(vec![
            SemanticNode::Symbol { glyph_key: key_a, role: SymbolRole::Ordinary },
            SemanticNode::Symbol { glyph_key: key_b, role: SymbolRole::Ordinary },
        ]);
        
        let layout = engine.layout_node(&node, MathStyle::Text).await.unwrap();
        
        assert_eq!(layout.width(), Fixed::from_f64(22.0));
        assert_eq!(layout.height(), Fixed::from_f64(9.0));
        assert_eq!(layout.depth(), Fixed::from_f64(2.0));
    }
}
