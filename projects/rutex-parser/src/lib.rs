use rutex_types::{Result, MathSemanticTree};

pub mod parser;
pub mod lexer;
pub mod macro_system;

pub use parser::Parser;
pub use lexer::{Token, Tokenizer};

pub fn parse(input: &str) -> Result<MathSemanticTree> {
    let mut parser = Parser::new(input);
    parser.parse()
}
