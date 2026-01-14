use serde::{Serialize, Deserialize};
pub use rutex_types::{
    RuTeXError, Result, Fixed, MathStyle, SemanticNode, GlyphKey, SpacingRule, LineStyle, Alignment, SymbolRole, FontStyle
};
use rutex_font::{FontMetricsSystem, GlyphMetrics as FontGlyphMetrics, MathConstant};
use std::sync::Arc;
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
    pub children: Vec<LayoutNode>,
    pub shift: Fixed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VBox {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub children: Vec<LayoutNode>,
    pub shift: Fixed,
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

pub struct LayoutEngine {
    font_system: FontMetricsSystem,
    base_size: Fixed,
}

impl LayoutEngine {
    pub fn new(font_system: FontMetricsSystem) -> Self {
        Self { 
            font_system,
            base_size: Fixed::from_f64(10.0),
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
                    for node in content {
                        children.push(self.layout_node(node, style).await?);
                        if let Some(glue) = self.get_spacing_glue(*spacing) {
                            children.push(LayoutNode::Glue(glue));
                        }
                    }
                    Ok(self.pack_hbox(children, None))
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
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;
        
        let metrics_font = self.font_system.get_metrics(key).await?;
        
        let metrics = GlyphMetrics {
            width: metrics_font.width * size,
            height: metrics_font.height * size,
            depth: metrics_font.depth * size,
            italic_correction: metrics_font.italic_correction * size,
        };

        Ok(LayoutNode::Glyph(Glyph {
            char: key.char,
            font_family: key.font_family.clone().unwrap_or_else(|| "default".to_string()),
            size,
            metrics,
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

    fn pack_hbox(&self, children: Vec<LayoutNode>, _alignment: Option<Alignment>) -> LayoutNode {
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
            children,
            shift: Fixed::ZERO,
        }))
    }

    fn pack_vbox(&self, children: Vec<LayoutNode>, _alignment: Alignment) -> LayoutNode {
        let mut width = Fixed::ZERO;
        let mut height = Fixed::ZERO;
        let mut depth = Fixed::ZERO;

        if let Some(last) = children.last() {
            depth = last.depth();
            
            let mut current_y = Fixed::ZERO;
            for child in children.iter().rev().skip(1) {
                current_y = current_y + child.depth() + child.height();
                width = width.max(child.width());
            }
            height = current_y + last.height();
            width = width.max(last.width());
        }

        LayoutNode::VBox(Box::new(VBox {
            width,
            height,
            depth,
            children,
            shift: Fixed::ZERO,
        }))
    }

    async fn layout_fraction(&self, num: &SemanticNode, den: &SemanticNode, line: LineStyle, style: MathStyle) -> Result<LayoutNode> {
        let next_style = match style {
            MathStyle::Display => MathStyle::Text,
            _ => MathStyle::Script,
        };

        let num_layout = self.layout_node(num, next_style).await?;
        let den_layout = self.layout_node(den, next_style).await?;

        let width = num_layout.width().max(den_layout.width());
        
        let num_centered = self.pack_hbox(vec![
            LayoutNode::Kern((width - num_layout.width()) / 2.0),
            num_layout,
        ], None);
        
        let den_centered = self.pack_hbox(vec![
            LayoutNode::Kern((width - den_layout.width()) / 2.0),
            den_layout,
        ], None);

        let den_centered_height = den_centered.height();

        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        let axis_height = self.font_system.get_math_constant(family, MathConstant::AxisHeight).await.unwrap_or(Fixed::from_f64(0.25)) * size;
        let rule_thickness = self.font_system.get_math_constant(family, MathConstant::FractionRuleThickness).await.unwrap_or(Fixed::from_f64(0.04)) * size;
        
        let (num_shift, den_shift, gap_min) = if style == MathStyle::Display {
            (
                self.font_system.get_math_constant(family, MathConstant::FractionNumeratorDisplayStyleShiftUp).await.unwrap_or(Fixed::from_f64(0.6)) * size,
                self.font_system.get_math_constant(family, MathConstant::FractionDenominatorDisplayStyleShiftDown).await.unwrap_or(Fixed::from_f64(0.4)) * size,
                self.font_system.get_math_constant(family, MathConstant::FractionNumDisplayStyleGapMin).await.unwrap_or(Fixed::from_f64(0.3)) * size
            )
        } else {
            (
                self.font_system.get_math_constant(family, MathConstant::FractionNumeratorShiftUp).await.unwrap_or(Fixed::from_f64(0.4)) * size,
                self.font_system.get_math_constant(family, MathConstant::FractionDenominatorShiftDown).await.unwrap_or(Fixed::from_f64(0.3)) * size,
                self.font_system.get_math_constant(family, MathConstant::FractionNumeratorGapMin).await.unwrap_or(Fixed::from_f64(0.1)) * size
            )
        };

        let mut children = Vec::new();
        children.push(num_centered);
        
        let num_gap = (num_shift - axis_height - (rule_thickness / 2.0)).max(gap_min);
        children.push(LayoutNode::Kern(num_gap));

        if matches!(line, LineStyle::Solid) {
            children.push(LayoutNode::Rule {
                width,
                height: rule_thickness,
                depth: Fixed::ZERO,
            });
        } else {
            children.push(LayoutNode::Kern(rule_thickness));
        }

        let den_gap = (den_shift + axis_height - (rule_thickness / 2.0)).max(gap_min);
        children.push(LayoutNode::Kern(den_gap));
        children.push(den_centered);

        let fraction_vbox = match self.pack_vbox(children, Alignment::Center) {
            LayoutNode::VBox(mut v) => {
                let dist_to_rule = den_centered_height + den_gap + (rule_thickness / 2.0);
                v.shift = axis_height - dist_to_rule;
                LayoutNode::VBox(v)
            }
            node => node,
        };

        Ok(fraction_vbox)
    }

    async fn layout_radical(&self, _degree: Option<&SemanticNode>, radicand: &SemanticNode, style: MathStyle) -> Result<LayoutNode> {
        let radicand_layout = self.layout_node(radicand, style).await?;
        
        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;

        let rule_thickness = self.font_system.get_math_constant(family, MathConstant::RadicalRuleThickness).await.unwrap_or(Fixed::from_f64(0.05)) * size;
        let vertical_gap = if style == MathStyle::Display {
            self.font_system.get_math_constant(family, MathConstant::RadicalDisplayStyleVerticalGap).await.unwrap_or(Fixed::from_f64(0.3)) * size
        } else {
            self.font_system.get_math_constant(family, MathConstant::RadicalVerticalGap).await.unwrap_or(Fixed::from_f64(0.1)) * size
        };

        let radical_key = GlyphKey {
            char: '√',
            font_family: None,
            style: rutex_types::FontStyle::Normal,
        };
        let radical_sym = self.layout_symbol(&radical_key, style).await?;
        
        let mut vbox_children = Vec::new();
        
        vbox_children.push(LayoutNode::Rule {
            width: radicand_layout.width(),
            height: rule_thickness,
            depth: Fixed::ZERO,
        });
        
        vbox_children.push(LayoutNode::Kern(vertical_gap));
        vbox_children.push(radicand_layout);
        
        let radicand_with_bar = self.pack_vbox(vbox_children, Alignment::Left);
        
        let mut hbox_children = Vec::new();
        hbox_children.push(radical_sym);
        hbox_children.push(radicand_with_bar);
        
        Ok(self.pack_hbox(hbox_children, None))
    }

    async fn layout_subsup(&self, base: &SemanticNode, sub: Option<&SemanticNode>, sup: Option<&SemanticNode>, style: MathStyle) -> Result<LayoutNode> {
        let base_layout = self.layout_node(base, style).await?;
        
        let next_style = match style {
            MathStyle::Display | MathStyle::Text => MathStyle::Script,
            _ => MathStyle::ScriptScript,
        };

        let family = "default";
        let style_scale = self.get_style_scale(style);
        let size = self.base_size * style_scale;
        let mut script_children: Vec<LayoutNode> = Vec::new();
        
        if let Some(sup_node) = sup {
            let sup_layout = self.layout_node(sup_node, next_style).await?;
            let sup_shift = self.font_system.get_math_constant(family, MathConstant::SuperscriptShiftUp).await.unwrap_or(Fixed::from_f64(0.5)) * size;
            
            let mut sup_vbox = Vec::new();
            sup_vbox.push(sup_layout);
            sup_vbox.push(LayoutNode::Kern(sup_shift));
            
            script_children.push(self.pack_vbox(sup_vbox, Alignment::Left));
        }
        
        if let Some(sub_node) = sub {
            let sub_layout = self.layout_node(sub_node, next_style).await?;
            let sub_shift = self.font_system.get_math_constant(family, MathConstant::SubscriptShiftDown).await.unwrap_or(Fixed::from_f64(0.3)) * size;
            
            let mut sub_vbox = Vec::new();
            sub_vbox.push(LayoutNode::Kern(sub_shift));
            sub_vbox.push(sub_layout);
            
            script_children.push(self.pack_vbox(sub_vbox, Alignment::Left));
        }

        let mut children = Vec::new();
        children.push(base_layout);
        
        if !script_children.is_empty() {
            let scripts = self.pack_vbox(script_children, Alignment::Left);
            children.push(scripts);
        }
        
        Ok(self.pack_hbox(children, None))
    }
}
