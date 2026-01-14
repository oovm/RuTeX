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
        LayoutNode::Kern(_) | LayoutNode::Glue(_) => {
            // Kerns and Glue don't render anything
        }
    }
    Ok(())
}

pub struct MathmlBackend {
    buffer: String,
    tag_stack: Vec<&'static str>,
}

impl MathmlBackend {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            tag_stack: Vec::new(),
        }
    }

    pub fn finish(mut self) -> String {
        let mut final_mml = String::from(r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="block">"#);
        
        // Add a top-level mrow to hold everything
        final_mml.push_str("<mrow>");
        final_mml.push_str(&self.buffer);
        
        // Close any remaining tags
        while let Some(tag) = self.tag_stack.pop() {
            final_mml.push_str("</");
            final_mml.push_str(tag);
            final_mml.push_str(">");
        }
        
        final_mml.push_str("</mrow>");
        final_mml.push_str("</math>");
        final_mml
    }
}

impl LayoutBackend for MathmlBackend {
    fn render_text(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        font_size: f64,
        font_family: Option<&str>,
    ) -> Result<()> {
        use std::fmt::Write;
        
        // Use <mpadded> for absolute positioning relative to the group
        write!(
            self.buffer,
            r#"<mpadded voffset="{}px" loffset="{}px">"#,
            y, x
        ).map_err(|e| RuTeXError::BackendError(e.to_string()))?;

        // Determine the MathML tag based on character type
        let tag = if text.chars().count() == 1 {
            let c = text.chars().next().unwrap();
            if c.is_numeric() {
                "mn"
            } else if c.is_alphabetic() {
                "mi"
            } else {
                "mo"
            }
        } else {
            "mtext"
        };

        let mut style = format!("font-size: {}px;", font_size);
        if let Some(family) = font_family {
            write!(style, " font-family: {};", family).unwrap();
        }

        write!(
            self.buffer,
            r#"<{tag} style="{}">{}</{tag}>"#,
            style, escape_xml(text)
        ).map_err(|e| RuTeXError::BackendError(e.to_string()))?;
        
        self.buffer.push_str("</mpadded>");
        Ok(())
    }

    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        use std::fmt::Write;
        // Use <mspace> with a background color for rules
        write!(
            self.buffer,
            r#"<mpadded voffset="{}px" loffset="{}px"><mspace width="{}px" height="{}px" style="background: currentColor;" /></mpadded>"#,
            y, x, w, h
        )
        .map_err(|e| RuTeXError::BackendError(e.to_string()))
    }

    fn render_path(&mut self, _d: &str) -> Result<()> {
        // MathML doesn't have a direct equivalent for SVG paths.
        // For now, we ignore paths or could potentially embed SVG inside MathML using <semantics>
        Ok(())
    }

    fn start_group(&mut self, transform: Option<&str>) -> Result<()> {
        use std::fmt::Write;
        
        if let Some(t) = transform {
            if t.starts_with("translate(") {
                // Parse translate(x, y)
                let content = &t[10..t.len() - 1];
                let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    let tx: f64 = parts[0].parse().unwrap_or(0.0);
                    let ty: f64 = parts[1].parse().unwrap_or(0.0);
                    
                    write!(self.buffer, r#"<mpadded voffset="{}px" loffset="{}px">"#, ty, tx)
                        .map_err(|e| RuTeXError::BackendError(e.to_string()))?;
                    self.tag_stack.push("mpadded");
                    return Ok(());
                }
            }
        }
        
        self.buffer.push_str("<mrow>");
        self.tag_stack.push("mrow");
        Ok(())
    }

    fn end_group(&mut self) -> Result<()> {
        if let Some(tag) = self.tag_stack.pop() {
            use std::fmt::Write;
            write!(self.buffer, "</{}>", tag)
                .map_err(|e| RuTeXError::BackendError(e.to_string()))
        } else {
            Err(RuTeXError::BackendError("No group to end".to_string()))
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mathml_basic() -> Result<()> {
        let mut backend = MathmlBackend::new();
        backend.render_rect(0.0, 0.0, 100.0, 50.0)?;
        backend.render_text("123", 10.0, 20.0, 12.0, None)?;
        
        let mml = backend.finish();
        assert!(mml.contains(r#"<math"#));
        assert!(mml.contains(r#"<mspace width="100px" height="50px""#));
        assert!(mml.contains(r#"<mtext style="font-size: 12px;">123</mtext>"#));
        Ok(())
    }

    #[test]
    fn test_mathml_groups() -> Result<()> {
        let mut backend = MathmlBackend::new();
        backend.start_group(Some("translate(10, 20)"))?;
        backend.render_text("x", 0.0, 0.0, 10.0, None)?;
        backend.end_group()?;
        
        let mml = backend.finish();
        assert!(mml.contains(r#"<mpadded voffset="20px" loffset="10px">"#));
        assert!(mml.contains(r#"<mi style="font-size: 10px;">x</mi>"#));
        Ok(())
    }
}

