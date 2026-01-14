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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MathStyle {
    Display,
    Text,
    Script,
    ScriptScript,
}
