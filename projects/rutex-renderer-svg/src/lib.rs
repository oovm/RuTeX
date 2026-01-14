pub use rutex_types::{RuTeXError, Result};

pub trait LayoutBackend {
    // Basic interface for rendering
    fn render_text(&mut self, text: &str, x: f64, y: f64) -> Result<()>;
    fn render_rect(&mut self, x: f64, y: f64, w: f64, h: f64) -> Result<()>;
}
