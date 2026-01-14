use std::fmt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RuTeXError {
    ParseError {
        message: String,
        position: Option<usize>,
    },
    LayoutError(String),
    FontError {
        glyph: String,
        message: String,
    },
    BackendError(String),
    InternalError(String),
    IO(String),
}

impl RuTeXError {
    pub fn io_error(err: impl std::fmt::Display) -> Self {
        RuTeXError::IO(err.to_string())
    }
}

impl From<std::io::Error> for RuTeXError {
    fn from(err: std::io::Error) -> Self {
        RuTeXError::IO(err.to_string())
    }
}

impl fmt::Display for RuTeXError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuTeXError::ParseError { message, position } => {
                if let Some(pos) = position {
                    write!(f, "Parse Error at position {}: {}", pos, message)
                } else {
                    write!(f, "Parse Error: {}", message)
                }
            }
            RuTeXError::LayoutError(msg) => write!(f, "Layout Error: {}", msg),
            RuTeXError::FontError { glyph, message } => {
                write!(f, "Font Error (Glyph: {}): {}", glyph, message)
            }
            RuTeXError::BackendError(msg) => write!(f, "Backend Error: {}", msg),
            RuTeXError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
            RuTeXError::IO(msg) => write!(f, "IO Error: {}", msg),
        }
    }
}

impl std::error::Error for RuTeXError {}

pub type Result<T> = std::result::Result<T, RuTeXError>;

// --- Math Semantic Tree ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MathSemanticTree {
    pub root: SemanticNode,
    // TODO: Add source mapping for debugging
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SemanticNode {
    Sequence(Vec<SemanticNode>),
    HorizontalBox {
        content: Vec<SemanticNode>,
        spacing: SpacingRule,
    },
    Paragraph {
        content: Vec<SemanticNode>,
        width: Fixed,
    },
    VerticalBox {
        content: Vec<SemanticNode>,
        alignment: Alignment,
    },
    Fraction {
        num: Box<SemanticNode>,
        den: Box<SemanticNode>,
        line: LineStyle,
    },
    Radical {
        degree: Option<Box<SemanticNode>>,
        radicand: Box<SemanticNode>,
    },
    Symbol {
        glyph_key: GlyphKey,
        role: SymbolRole,
    },
    Text(String),
    Subscript {
        base: Box<SemanticNode>,
        sub: Box<SemanticNode>,
    },
    Superscript {
        base: Box<SemanticNode>,
        sup: Box<SemanticNode>,
    },
    SubSuperscript {
        base: Box<SemanticNode>,
        sub: Box<SemanticNode>,
        sup: Box<SemanticNode>,
    },
    Matrix {
        rows: Vec<Vec<SemanticNode>>,
        row_spacing: Fixed,
        col_spacing: Fixed,
        alignment: Alignment,
    },
    Delimited {
        left: Option<GlyphKey>,
        right: Option<GlyphKey>,
        content: Box<SemanticNode>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SpacingRule {
    None,
    Thin,
    Medium,
    Thick,
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LineStyle {
    Solid,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub char: char,
    pub font_family: Option<String>,
    pub style: FontStyle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
    Bold,
    Math,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SymbolRole {
    Ordinary,
    Operator,
    Binary,
    Relation,
    Opening,
    Closing,
    Punctuation,
    Inner,
    LargeOperator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MathStyle {
    Display,
    Text,
    Script,
    ScriptScript,
}

/// Fixed-point number for precise layout calculations.
/// Using I28F4 as suggested in roadmap (or adaptable to TeX's 16.16).
/// Here we use i32 with 16 bits for fractional part (TeX style) for better compatibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Fixed(pub i32);

impl Fixed {
    pub const ZERO: Fixed = Fixed(0);
    pub const ONE: Fixed = Fixed(65536);
    pub const SCALE: i32 = 65536;

    pub fn from_f64(f: f64) -> Self {
        Fixed((f * Self::SCALE as f64) as i32)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }
}

impl std::ops::Add for Fixed {
    type Output = Self;
    fn add(self, other: Self) -> Self { Fixed(self.0 + other.0) }
}

impl std::ops::Sub for Fixed {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Fixed(self.0 - other.0) }
}

impl std::ops::Mul for Fixed {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Fixed(((self.0 as i64 * rhs.0 as i64) / Self::SCALE as i64) as i32)
    }
}

impl std::ops::Div for Fixed {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            Fixed(0) // Should probably be an error or infinity
        } else {
            Fixed(((self.0 as i64 * Self::SCALE as i64) / rhs.0 as i64) as i32)
        }
    }
}

impl std::ops::Mul<f64> for Fixed {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self { Fixed((self.0 as f64 * rhs) as i32) }
}

impl std::ops::Div<f64> for Fixed {
    type Output = Self;
    fn div(self, rhs: f64) -> Self { Fixed((self.0 as f64 / rhs) as i32) }
}
