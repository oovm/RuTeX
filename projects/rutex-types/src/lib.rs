use std::fmt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
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
