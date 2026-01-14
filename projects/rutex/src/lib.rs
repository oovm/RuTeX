pub use rutex_types::{RuTeXError, Result};
pub use rutex_parser as parser;
pub use rutex_layout as layout;
pub use rutex_font as font;
pub use rutex_renderer_svg as renderer;

pub fn render(tex: &str) -> Result<String> {
    // Orchestration logic will go here
    Ok(format!("Rendering: {}", tex))
}
