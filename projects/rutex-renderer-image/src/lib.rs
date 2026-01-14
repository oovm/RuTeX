use tiny_skia::{PathBuilder, Pixmap, Paint, FillRule, Transform, Color, Rect};
pub use rutex_types::{RuTeXError, Result};
pub use rutex_layout::{LayoutNode, Glyph};
use std::collections::HashMap;
use std::sync::Arc;

pub trait LayoutBackend {
    fn render_text(&mut self, text: &str, x: f64, y: f64, font_size: f64, font_family: Option<&str>) -> Result<()>;
    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()>;
    fn render_path(&mut self, d: &str) -> Result<()>;
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
                (height.to_f64() + depth.to_f64()),
            )?;
        }
        LayoutNode::Kern(_) | LayoutNode::Glue(_) => {}
    }
    Ok(())
}

pub struct ImageBackend {
    pixmap: Pixmap,
    transform_stack: Vec<Transform>,
    current_transform: Transform,
    font_data: HashMap<String, Arc<Vec<u8>>>,
}

impl ImageBackend {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        Some(Self {
            pixmap: Pixmap::new(width, height)?,
            transform_stack: Vec::new(),
            current_transform: Transform::identity(),
            font_data: HashMap::new(),
        })
    }

    pub fn add_font(&mut self, family: String, data: Arc<Vec<u8>>) {
        self.font_data.insert(family, data);
    }

    pub fn finish(self) -> Vec<u8> {
        self.pixmap.encode_png().unwrap_or_default()
    }
}

struct OutlineBuilder<'a>(&'a mut PathBuilder);

impl<'a> ttf_parser::OutlineBuilder for OutlineBuilder<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

impl LayoutBackend for ImageBackend {
    fn render_text(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: Option<&str>,
    ) -> Result<()> {
        let family = font_family.unwrap_or("default");
        let data = self.font_data.get(family).ok_or_else(|| {
            RuTeXError::BackendError(format!("Font family '{}' not found", family))
        })?;

        let face = ttf_parser::Face::parse(data, 0).map_err(|e| {
            RuTeXError::BackendError(format!("Failed to parse font: {}", e))
        })?;

        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.anti_alias = true;

        for c in text.chars() {
            let glyph_id = face.glyph_index(c).ok_or_else(|| {
                RuTeXError::BackendError(format!("Glyph for '{}' not found", c))
            })?;

            let mut builder = PathBuilder::new();
            let mut outline_builder = OutlineBuilder(&mut builder);
            
            if let Some(_rect) = face.outline_glyph(glyph_id, &mut outline_builder) {
                if let Some(path) = builder.finish() {
                    let scale = (font_size / face.units_per_em() as f64) as f32;
                    // TTF coordinates are Y-up, tiny-skia is Y-down
                    let transform = self.current_transform
                        .post_translate(x as f32, y as f32)
                        .pre_scale(scale, -scale);
                    
                    self.pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
                }
            }
            // Note: Horizontal advance is not handled here as render_layout_node handles positioning
        }

        Ok(())
    }

    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.anti_alias = true;

        let rect = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
            .ok_or_else(|| RuTeXError::BackendError("Invalid rect dimensions".to_string()))?;

        self.pixmap.fill_rect(rect, &paint, self.current_transform, None);
        Ok(())
    }

    fn render_path(&mut self, _d: &str) -> Result<()> {
        // SVG path parsing would be needed here, skipping for simplicity
        // or using tiny_skia's path parser if available
        Ok(())
    }

    fn start_group(&mut self, transform: Option<&str>) -> Result<()> {
        self.transform_stack.push(self.current_transform);
        if let Some(t) = transform {
            if t.starts_with("translate(") {
                let parts: Vec<&str> = t[10..t.len()-1].split(',').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    let tx: f32 = parts[0].parse().unwrap_or(0.0);
                    let ty: f32 = parts[1].parse().unwrap_or(0.0);
                    self.current_transform = self.current_transform.pre_translate(tx, ty);
                }
            }
        }
        Ok(())
    }

    fn end_group(&mut self) -> Result<()> {
        if let Some(prev) = self.transform_stack.pop() {
            self.current_transform = prev;
            Ok(())
            
        } else {
            Err(RuTeXError::BackendError("No group to end".to_string()))
        }
    }
}
