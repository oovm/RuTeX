use std::sync::Arc;
use dashmap::DashMap;
pub use rutex_types::{RuTeXError, Result};

pub struct GlyphMetrics {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
}

pub struct FontMetricsSystem {
    cache: Arc<DashMap<String, GlyphMetrics>>,
}

impl FontMetricsSystem {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub async fn get_metrics(&self, glyph_key: &str) -> Result<GlyphMetrics> {
        self.cache.get(glyph_key)
            .map(|m| GlyphMetrics { 
                width: m.width, 
                height: m.height, 
                depth: m.depth 
            })
            .ok_or_else(|| RuTeXError::FontError { 
                glyph: glyph_key.to_string(), 
                message: "Glyph not found in metrics system".to_string() 
            })
    }
}
