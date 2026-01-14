use serde::{Serialize, Deserialize};
use std::collections::HashMap;
pub use rutex_types::{RuTeXError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct MathSemanticTree {
    pub root: SemanticNode,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SemanticNode {
    Sequence(Vec<SemanticNode>),
    HorizontalBox { content: Vec<SemanticNode>, spacing: SpacingRule },
    VerticalBox { content: Vec<SemanticNode>, alignment: Alignment },
    Fraction { num: Box<SemanticNode>, den: Box<SemanticNode>, line: LineStyle },
    Radical { degree: Option<Box<SemanticNode>>, radicand: Box<SemanticNode> },
    Symbol { glyph_key: String, role: SymbolRole },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SpacingRule { Default, Tight, Loose }

#[derive(Debug, Serialize, Deserialize)]
pub enum Alignment { Center, Left, Right }

#[derive(Debug, Serialize, Deserialize)]
pub enum LineStyle { Solid, None }

#[derive(Debug, Serialize, Deserialize)]
pub enum SymbolRole { Ordinary, Operator, Binary, Relation, Open, Close, Punctuation }

pub struct ParseContext {
    pub macros: HashMap<String, String>,
    pub math_style: MathStyle,
    pub environment_stack: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MathStyle { Display, Text, Script, ScriptScript }

pub enum ParseEffect {
    MacroExpansion(String, Vec<String>),
    StyleChange(MathStyle),
    BeginEnv(String),
    EndEnv,
}
