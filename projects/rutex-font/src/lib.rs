use std::sync::Arc;
use dashmap::DashMap;
use async_trait::async_trait;
pub use rutex_types::{RuTeXError, Result, GlyphKey, Fixed};

#[derive(Debug, Clone, Copy)]
pub enum MathConstant {
    ScriptPercentScaleDown,
    ScriptScriptPercentScaleDown,
    DelimitedSubFormulaMinHeight,
    DisplayOperatorMinHeight,
    MathLeading,
    AxisHeight,
    AccentBaseHeight,
    FlattenedAccentBaseHeight,
    SubscriptShiftDown,
    SubscriptTopMax,
    SubscriptBaselineDropMin,
    SuperscriptShiftUp,
    SuperscriptShiftUpCramped,
    SuperscriptBottomMin,
    SuperscriptBaselineDropMax,
    SubSuperscriptGapMin,
    SuperscriptBottomMaxWithSubscript,
    SpaceAfterScript,
    UpperLimitGapMin,
    UpperLimitBaselineRiseMin,
    LowerLimitGapMin,
    LowerLimitBaselineDropMin,
    StackTopShiftUp,
    StackTopDisplayStyleShiftUp,
    StackBottomShiftDown,
    StackBottomDisplayStyleShiftDown,
    StackGapMin,
    StackDisplayStyleGapMin,
    StretchStackTopShiftUp,
    StretchStackBottomShiftDown,
    StretchStackGapAboveMin,
    StretchStackGapBelowMin,
    FractionNumeratorShiftUp,
    FractionNumeratorDisplayStyleShiftUp,
    FractionDenominatorShiftDown,
    FractionDenominatorDisplayStyleShiftDown,
    FractionNumeratorGapMin,
    FractionNumDisplayStyleGapMin,
    FractionRuleThickness,
    FractionDenominatorGapMin,
    FractionDenomDisplayStyleGapMin,
    SkewedFractionHorizontalGap,
    SkewedFractionVerticalGap,
    OverbarVerticalGap,
    OverbarRuleThickness,
    OverbarExtraAscender,
    UnderbarVerticalGap,
    UnderbarRuleThickness,
    UnderbarExtraDescender,
    RadicalVerticalGap,
    RadicalDisplayStyleVerticalGap,
    RadicalRuleThickness,
    RadicalExtraAscender,
    RadicalKernBeforeDegree,
    RadicalKernAfterDegree,
    RadicalDegreeBottomRaisePercent,
}

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

pub struct FileFontLoader {
    base_path: std::path::PathBuf,
}

impl FileFontLoader {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }
}

#[async_trait]
impl FontLoader for FileFontLoader {
    async fn load_font_data(&self, family: &str) -> Result<Arc<Vec<u8>>> {
        let path = self.base_path.join(format!("{}.ttf", family));
        let data = std::fs::read(&path).map_err(|e| RuTeXError::FontError {
            glyph: family.to_string(),
            message: format!("Failed to read font file {:?}: {}", path, e),
        })?;
        Ok(Arc::new(data))
    }
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

    pub fn insert_metrics(&self, key: GlyphKey, metrics: GlyphMetrics) {
        self.metrics_cache.insert(key, metrics);
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

        let constants = math.constants.ok_or_else(|| RuTeXError::FontError {
            glyph: family.to_string(),
            message: "No math constants in font".to_string(),
        })?;

        let upem = face.units_per_em() as f64;
        let value = match constant {
            MathConstant::ScriptPercentScaleDown => constants.script_percent_scale_down() as i32,
            MathConstant::ScriptScriptPercentScaleDown => constants.script_script_percent_scale_down() as i32,
            MathConstant::DelimitedSubFormulaMinHeight => constants.delimited_sub_formula_min_height() as i32,
            MathConstant::DisplayOperatorMinHeight => constants.display_operator_min_height() as i32,
            MathConstant::MathLeading => constants.math_leading().value as i32,
            MathConstant::AxisHeight => constants.axis_height().value as i32,
            MathConstant::AccentBaseHeight => constants.accent_base_height().value as i32,
            MathConstant::FlattenedAccentBaseHeight => constants.flattened_accent_base_height().value as i32,
            MathConstant::SubscriptShiftDown => constants.subscript_shift_down().value as i32,
            MathConstant::SubscriptTopMax => constants.subscript_top_max().value as i32,
            MathConstant::SubscriptBaselineDropMin => constants.subscript_baseline_drop_min().value as i32,
            MathConstant::SuperscriptShiftUp => constants.superscript_shift_up().value as i32,
            MathConstant::SuperscriptShiftUpCramped => constants.superscript_shift_up_cramped().value as i32,
            MathConstant::SuperscriptBottomMin => constants.superscript_bottom_min().value as i32,
            MathConstant::SuperscriptBaselineDropMax => constants.superscript_baseline_drop_max().value as i32,
            MathConstant::SubSuperscriptGapMin => constants.sub_superscript_gap_min().value as i32,
            MathConstant::SuperscriptBottomMaxWithSubscript => constants.superscript_bottom_max_with_subscript().value as i32,
            MathConstant::SpaceAfterScript => constants.space_after_script().value as i32,
            MathConstant::UpperLimitGapMin => constants.upper_limit_gap_min().value as i32,
            MathConstant::UpperLimitBaselineRiseMin => constants.upper_limit_baseline_rise_min().value as i32,
            MathConstant::LowerLimitGapMin => constants.lower_limit_gap_min().value as i32,
            MathConstant::LowerLimitBaselineDropMin => constants.lower_limit_baseline_drop_min().value as i32,
            MathConstant::StackTopShiftUp => constants.stack_top_shift_up().value as i32,
            MathConstant::StackTopDisplayStyleShiftUp => constants.stack_top_display_style_shift_up().value as i32,
            MathConstant::StackBottomShiftDown => constants.stack_bottom_shift_down().value as i32,
            MathConstant::StackBottomDisplayStyleShiftDown => constants.stack_bottom_display_style_shift_down().value as i32,
            MathConstant::StackGapMin => constants.stack_gap_min().value as i32,
            MathConstant::StackDisplayStyleGapMin => constants.stack_display_style_gap_min().value as i32,
            MathConstant::StretchStackTopShiftUp => constants.stretch_stack_top_shift_up().value as i32,
            MathConstant::StretchStackBottomShiftDown => constants.stretch_stack_bottom_shift_down().value as i32,
            MathConstant::StretchStackGapAboveMin => constants.stretch_stack_gap_above_min().value as i32,
            MathConstant::StretchStackGapBelowMin => constants.stretch_stack_gap_below_min().value as i32,
            MathConstant::FractionNumeratorShiftUp => constants.fraction_numerator_shift_up().value as i32,
            MathConstant::FractionNumeratorDisplayStyleShiftUp => constants.fraction_numerator_display_style_shift_up().value as i32,
            MathConstant::FractionDenominatorShiftDown => constants.fraction_denominator_shift_down().value as i32,
            MathConstant::FractionDenominatorDisplayStyleShiftDown => constants.fraction_denominator_display_style_shift_down().value as i32,
            MathConstant::FractionNumeratorGapMin => constants.fraction_numerator_gap_min().value as i32,
            MathConstant::FractionNumDisplayStyleGapMin => constants.fraction_num_display_style_gap_min().value as i32,
            MathConstant::FractionRuleThickness => constants.fraction_rule_thickness().value as i32,
            MathConstant::FractionDenominatorGapMin => constants.fraction_denominator_gap_min().value as i32,
            MathConstant::FractionDenomDisplayStyleGapMin => constants.fraction_denom_display_style_gap_min().value as i32,
            MathConstant::SkewedFractionHorizontalGap => constants.skewed_fraction_horizontal_gap().value as i32,
            MathConstant::SkewedFractionVerticalGap => constants.skewed_fraction_vertical_gap().value as i32,
            MathConstant::OverbarVerticalGap => constants.overbar_vertical_gap().value as i32,
            MathConstant::OverbarRuleThickness => constants.overbar_rule_thickness().value as i32,
            MathConstant::OverbarExtraAscender => constants.overbar_extra_ascender().value as i32,
            MathConstant::UnderbarVerticalGap => constants.underbar_vertical_gap().value as i32,
            MathConstant::UnderbarRuleThickness => constants.underbar_rule_thickness().value as i32,
            MathConstant::UnderbarExtraDescender => constants.underbar_extra_descender().value as i32,
            MathConstant::RadicalVerticalGap => constants.radical_vertical_gap().value as i32,
            MathConstant::RadicalDisplayStyleVerticalGap => constants.radical_display_style_vertical_gap().value as i32,
            MathConstant::RadicalRuleThickness => constants.radical_rule_thickness().value as i32,
            MathConstant::RadicalExtraAscender => constants.radical_extra_ascender().value as i32,
            MathConstant::RadicalKernBeforeDegree => constants.radical_kern_before_degree().value as i32,
            MathConstant::RadicalKernAfterDegree => constants.radical_kern_after_degree().value as i32,
            MathConstant::RadicalDegreeBottomRaisePercent => constants.radical_degree_bottom_raise_percent() as i32,
        };

        // Normalize by UPEM (1.0 = 1 em)
        // Some constants are percentages (e.g. ScriptPercentScaleDown), they shouldn't be normalized by UPEM.
        // Percentages in OpenType Math are usually 0-100.
        match constant {
            MathConstant::ScriptPercentScaleDown | 
            MathConstant::ScriptScriptPercentScaleDown | 
            MathConstant::RadicalDegreeBottomRaisePercent => {
                Ok(Fixed::from_f64(value as f64 / 100.0))
            }
            _ => {
                Ok(Fixed::from_f64(value as f64 / upem))
            }
        }
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
                glyph: key.char.to_string(),
                message: format!("Failed to parse font: {}", e),
            })?;

        let glyph_id = face.glyph_index(key.char)
            .ok_or_else(|| RuTeXError::FontError {
                glyph: key.char.to_string(),
                message: format!("Glyph for '{}' not found in font", key.char),
            })?;

        let upem = face.units_per_em() as f64;
        let width = Fixed::from_f64(face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64 / upem);
        
        let bbox = face.glyph_bounding_box(glyph_id)
            .unwrap_or(ttf_parser::Rect { x_min: 0, y_min: 0, x_max: 0, y_max: 0 });

        let height = Fixed::from_f64(bbox.y_max as f64 / upem);
        let depth = Fixed::from_f64((-bbox.y_min).max(0) as f64 / upem);

        let mut italic_correction = Fixed::ZERO;
        
        if let Some(math) = face.tables().math {
            if let Some(glyph_info) = math.glyph_info {
                if let Some(it_corr) = glyph_info.italic_corrections {
                    if let Some(value) = it_corr.get(glyph_id) {
                        italic_correction = Fixed::from_f64(value.value as f64 / upem);
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
