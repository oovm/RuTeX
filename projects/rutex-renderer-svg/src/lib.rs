pub use rutex_types::{RuTeXError, Result};
pub use rutex_layout::LayoutNode;

pub trait LayoutBackend {
    // Basic interface for rendering
    fn render_text(&mut self, text: &str, x: f64, y: f64, font_size: f64, font_family: Option<&str>) -> Result<()>;
    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()>;
    fn render_path(&mut self, d: &str) -> Result<()>;
    
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
        LayoutNode::Kern(_amount) => {
            // Kerns don't render anything, they just affect positioning
            // which is handled by the parent box's current_x/current_y
        }
        LayoutNode::Glue(_glue) => {
            // Glue rendering is similar to kern in that it's just spacing
        }
    }
    Ok(())
}

pub struct SvgBackend {
    buffer: String,
    width: f64,
    height: f64,
    group_depth: usize,
}

impl SvgBackend {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            buffer: String::new(),
            width,
            height,
            group_depth: 0,
        }
    }

    pub fn finish(self) -> String {
        let mut final_svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            self.width, self.height, self.width, self.height
        );
        final_svg.push_str(&self.buffer);
        // Close any remaining groups just in case
        for _ in 0..self.group_depth {
            final_svg.push_str("</g>");
        }
        final_svg.push_str("</svg>");
        final_svg
    }
}

impl LayoutBackend for SvgBackend {
    fn render_text(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: Option<&str>,
    ) -> Result<()> {
        use std::fmt::Write;
        let font_family_attr = if let Some(family) = font_family {
            format!(r#" font-family="{}""#, family)
        } else {
            String::new()
        };
        write!(
            self.buffer,
            r#"<text x="{}" y="{}" font-size="{}"{} fill="currentColor">{}</text>"#,
            x, y, font_size, font_family_attr, text
        )
        .map_err(|e| RuTeXError::BackendError(e.to_string()))
    }

    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        use std::fmt::Write;
        write!(
            self.buffer,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="currentColor" />"#,
            x, y, w, h
        )
        .map_err(|e| RuTeXError::BackendError(e.to_string()))
    }

    fn render_path(&mut self, d: &str) -> Result<()> {
        use std::fmt::Write;
        write!(
            self.buffer,
            r#"<path d="{}" fill="currentColor" />"#,
            d
        )
        .map_err(|e| RuTeXError::BackendError(e.to_string()))
    }

    fn start_group(&mut self, transform: Option<&str>) -> Result<()> {
        use std::fmt::Write;
        if let Some(t) = transform {
            write!(self.buffer, r#"<g transform="{}">"#, t)
        } else {
            write!(self.buffer, "<g>")
        }
        .map_err(|e| RuTeXError::BackendError(e.to_string()))?;
        self.group_depth += 1;
        Ok(())
    }

    fn end_group(&mut self) -> Result<()> {
        if self.group_depth > 0 {
            self.buffer.push_str("</g>");
            self.group_depth -= 1;
            Ok(())
        } else {
            Err(RuTeXError::BackendError("No group to end".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_basic() -> Result<()> {
        let mut backend = SvgBackend::new(100.0, 50.0);
        backend.render_rect(0.0, 0.0, 100.0, 50.0)?;
        backend.render_text("Hello", 10.0, 20.0, 12.0, Some("Arial"))?;
        
        let svg = backend.finish();
        assert!(svg.contains(r#"width="100" height="50""#));
        assert!(svg.contains(r#"<rect x="0" y="0" width="100" height="50""#));
        assert!(svg.contains(r#"<text x="10" y="20" font-size="12" font-family="Arial""#));
        assert!(svg.contains("Hello"));
        Ok(())
    }

    #[test]
    fn test_groups() -> Result<()> {
        let mut backend = SvgBackend::new(100.0, 100.0);
        backend.start_group(Some("translate(10, 10)"))?;
        backend.render_rect(0.0, 0.0, 10.0, 10.0)?;
        backend.end_group()?;
        
        let svg = backend.finish();
        assert!(svg.contains(r#"<g transform="translate(10, 10)">"#));
        assert!(svg.contains("</g>"));
        Ok(())
    }

    #[test]
    fn test_unclosed_group() -> Result<()> {
        let mut backend = SvgBackend::new(100.0, 100.0);
        backend.start_group(None)?;
        backend.render_rect(0.0, 0.0, 10.0, 10.0)?;
        // Not calling end_group
        
        let svg = backend.finish();
        assert!(svg.contains("<g>"));
        assert!(svg.contains("</g>")); // finish() should close it
        Ok(())
    }

    #[test]
    fn test_render_layout_node() -> Result<()> {
        use rutex_layout::{Glyph, GlyphMetrics, HBox};
        use rutex_types::Fixed;

        let glyph = LayoutNode::Glyph(Glyph {
            char: 'A',
            font_family: "Serif".to_string(),
            size: Fixed::from_f64(12.0),
            metrics: GlyphMetrics {
                width: Fixed::from_f64(8.0),
                height: Fixed::from_f64(10.0),
                depth: Fixed::from_f64(0.0),
                italic_correction: Fixed::ZERO,
            },
        });

        let hbox = LayoutNode::HBox(Box::new(HBox {
            width: Fixed::from_f64(8.0),
            height: Fixed::from_f64(10.0),
            depth: Fixed::from_f64(0.0),
            shift: Fixed::ZERO,
            children: vec![glyph],
            glue_set: 0.0,
        }));

        let mut backend = SvgBackend::new(100.0, 100.0);
        render_layout_node(&mut backend, &hbox, 10.0, 10.0)?;

        let svg = backend.finish();
        assert!(svg.contains(r#"translate(10, 10)"#));
        assert!(svg.contains(r#"font-family="Serif""#));
        assert!(svg.contains("A"));
        Ok(())
    }

    #[test]
    fn test_svg_generation() -> Result<()> {
        let mut backend = SvgBackend::new(100.0, 50.0);
        backend.render_text("Hello", 10.0, 30.0, 16.0, None)?;
        backend.render_rect(10.0, 35.0, 80.0, 2.0)?;
        
        backend.start_group(Some("translate(5,5)"))?;
        backend.render_path("M 0 0 L 10 10")?;
        backend.end_group()?;
        
        let svg = backend.finish();
        
        assert!(svg.contains(r#"width="100""#));
        assert!(svg.contains(r#"height="50""#));
        assert!(svg.contains(r#"<text x="10" y="30" font-size="16" fill="currentColor">Hello</text>"#));
        assert!(svg.contains(r#"<rect x="10" y="35" width="80" height="2" fill="currentColor" />"#));
        assert!(svg.contains(r#"<g transform="translate(5,5)">"#));
        assert!(svg.contains(r#"<path d="M 0 0 L 10 10" fill="currentColor" />"#));
        assert!(svg.contains("</g>"));
        Ok(())
    }
}
