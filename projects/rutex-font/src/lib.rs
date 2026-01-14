use std::sync::Arc;
use dashmap::DashMap;
use async_trait::async_trait;
pub use rutex_types::{RuTeXError, Result, GlyphKey, Fixed};
pub use ttf_parser::math::Constant as MathConstant;

#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub width: Fixed,
    pub height: Fixed,
    pub depth: Fixed,
    pub italic_correction: Fixed,
}

#[async_trait]
pub trait FontLoader: Send + Sync {
    async fn load_font_data(&self, family: &str) -> Result<Arc<Vec<u8>>>;
}

pub struct FontMetricsSystem {
    loader: Arc<dyn FontLoader>,
    font_data_cache: DashMap<String, Arc<Vec<u8>>>,
    metrics_cache: DashMap<GlyphKey, GlyphMetrics>,
}

impl FontMetricsSystem {
    pub fn new(loader: Arc<dyn FontLoader>) -> Self {
        Self {
            loader,
            font_data_cache: DashMap::new(),
            metrics_cache: DashMap::new(),
        }
    }

    pub async fn get_metrics(&self, key: &GlyphKey) -> Result<GlyphMetrics> {
        if let Some(metrics) = self.metrics_cache.get(key) {
            return Ok(*metrics);
        }

        let family = key.font_family.as_deref().unwrap_or("default");
        let data = self.get_font_data(family).await?;

        let metrics = self.parse_metrics(&data, key)?;
        self.metrics_cache.insert(key.clone(), metrics);
        
        Ok(metrics)
    }

    pub async fn get_math_constant(&self, family: &str, constant: MathConstant) -> Result<Fixed> {
        let data = self.get_font_data(family).await?;
        let face = ttf_parser::Face::parse(&data, 0)
            .map_err(|e| RuTeXError::FontError {
                glyph: family.to_string(),
                message: format!("Failed to parse font: {}", e),
            })?;

        let math = face.tables().math.ok_or_else(|| RuTeXError::FontError {
            glyph: family.to_string(),
            message: "No MATH table in font".to_string(),
        })?;

        let value = math.constants.and_then(|c| c.get(constant))
            .ok_or_else(|| RuTeXError::FontError {
                glyph: family.to_string(),
                message: format!("Constant {:?} not found", constant),
            })?;

        Ok(Fixed::from_f64(value.value as f64))
    }

    async fn get_font_data(&self, family: &str) -> Result<Arc<Vec<u8>>> {
        if let Some(data) = self.font_data_cache.get(family) {
            return Ok(data.clone());
        }
        let data = self.loader.load_font_data(family).await?;
        self.font_data_cache.insert(family.to_string(), data.clone());
        Ok(data)
    }

    fn parse_metrics(&self, data: &[u8], key: &GlyphKey) -> Result<GlyphMetrics> {
        let face = ttf_parser::Face::parse(data, 0)
            .map_err(|e| RuTeXError::FontError {
                glyph: format!("{:?}", key),
                message: format!("Failed to parse font: {}", e),
            })?;

        let glyph_id = face.glyph_index(key.char)
            .ok_or_else(|| RuTeXError::FontError {
                glyph: key.char.to_string(),
                message: format!("Glyph for '{}' not found in font", key.char),
            })?;

        let width = Fixed::from_f64(face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64);
        
        let bbox = face.glyph_bounding_box(glyph_id)
            .unwrap_or(ttf_parser::Rect { x_min: 0, y_min: 0, x_max: 0, y_max: 0 });

        // In TeX/Math layout:
        // height is the distance from baseline to top
        // depth is the distance from baseline to bottom (positive value)
        let height = Fixed::from_f64(bbox.y_max as f64);
        let depth = Fixed::from_f64((-bbox.y_min).max(0) as f64);

        let mut italic_correction = Fixed::ZERO;
        
        // Extract italic correction from MATH table if available
        if let Some(math) = face.tables().math {
            if let Some(glyph_info) = math.glyph_info {
                if let Some(it_corr) = glyph_info.italic_corrections {
                    if let Some(value) = it_corr.get(glyph_id) {
                        italic_correction = Fixed::from_f64(value.value as f64);
                    }
                }
            }
        }

        Ok(GlyphMetrics {
            width,
            height,
            depth,
            italic_correction,
        })
    }
}
