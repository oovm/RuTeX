pub use rutex_types::{RuTeXError, Result};
pub use rutex_layout::LayoutNode;
pub use rutex_renderer_svg::{SvgBackend, render_layout_node, LayoutBackend};

pub struct ImageBackend {
    svg_backend: SvgBackend,
    ppi: f32,
}

impl ImageBackend {
    pub fn new(width: f64, height: f64, ppi: f32) -> Self {
        Self {
            svg_backend: SvgBackend::new(width, height),
            ppi,
        }
    }

    pub fn finish(self) -> Result<Vec<u8>> {
        let svg_str = self.svg_backend.finish();
        svg_to_png(&svg_str, self.ppi)
    }
}

impl LayoutBackend for ImageBackend {
    fn render_text(&mut self, text: &str, x: f64, y: f64, font_size: f64, font_family: Option<&str>) -> Result<()> {
        self.svg_backend.render_text(text, x, y, font_size, font_family)
    }

    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        self.svg_backend.render_rect(x, y, w, h)
    }

    fn render_path(&mut self, d: &str) -> Result<()> {
        self.svg_backend.render_path(d)
    }

    fn start_group(&mut self, transform: Option<&str>) -> Result<()> {
        self.svg_backend.start_group(transform)
    }

    fn end_group(&mut self) -> Result<()> {
        self.svg_backend.end_group()
    }
}

pub fn render_to_png(node: &LayoutNode, ppi: f32) -> Result<Vec<u8>> {
    let width = node.width().to_f64();
    let height = (node.height() + node.depth()).to_f64();
    
    let mut backend = ImageBackend::new(width, height, ppi);
    render_layout_node(&mut backend, node, 0.0, node.height().to_f64())?;
    backend.finish()
}

fn svg_to_png(svg_str: &str, ppi: f32) -> Result<Vec<u8>> {
    let mut opt = resvg::usvg::Options::default();
    // usvg 0.42+ uses fontdb for text rendering. 
    // For now we use default options.
    
    let rtree = resvg::usvg::Tree::from_str(svg_str, &opt)
        .map_err(|e| RuTeXError::BackendError(format!("SVG parse error: {}", e)))?;
    
    // Calculate scale based on PPI (assuming 96 is default)
    let scale = ppi / 96.0;
    let width = (rtree.size().width() * scale as f64).round() as u32;
    let height = (rtree.size().height() * scale as f64).round() as u32;
    
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RuTeXError::BackendError("Failed to create pixmap".to_string()))?;
    
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&rtree, resvg::FitTo::Original, transform, pixmap.as_mut());
    
    pixmap.encode_png()
        .map_err(|e| RuTeXError::BackendError(format!("PNG encoding error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_basic() -> Result<()> {
        // Just a smoke test to see if it links and runs
        let mut backend = ImageBackend::new(100.0, 50.0, 96.0);
        backend.render_rect(0.0, 0.0, 100.0, 50.0)?;
        
        let png = backend.finish()?;
        assert!(!png.is_empty());
        Ok(())
    }
}
