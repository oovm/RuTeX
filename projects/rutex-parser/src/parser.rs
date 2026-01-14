use crate::lexer::{Token, Tokenizer};
use crate::macro_system::ParseContext;
use rutex_types::{
    MathSemanticTree, SemanticNode, Result, RuTeXError, 
    GlyphKey, FontStyle, SymbolRole
};
use std::iter::Peekable;

pub struct Parser<'a> {
    tokens: Peekable<Tokenizer<'a>>,
    context: ParseContext,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            tokens: Tokenizer::new(input).peekable(),
            context: ParseContext::default(),
        }
    }

    pub fn parse(&mut self) -> Result<MathSemanticTree> {
        let root = self.parse_sequence()?;
        Ok(MathSemanticTree { root })
    }

    fn parse_sequence(&mut self) -> Result<SemanticNode> {
        let mut nodes = Vec::new();
        while let Some(token_res) = self.tokens.peek() {
            if token_res.is_err() {
                return Err(RuTeXError::ParseError {
                    message: "Invalid token".to_string(),
                    position: None,
                });
            }
            
            let token = token_res.as_ref().unwrap();
            match token {
                Token::RBrace | Token::RBracket | Token::RParen => break,
                _ => {
                    let node = self.parse_node()?;
                    nodes.push(node);
                }
            }
        }

        if nodes.len() == 1 {
            Ok(nodes.pop().unwrap())
        } else {
            Ok(SemanticNode::Sequence(nodes))
        }
    }

    fn parse_node(&mut self) -> Result<SemanticNode> {
        let mut node = self.parse_primary()?;

        // Handle sub/superscripts
        loop {
            match self.tokens.peek() {
                Some(Ok(Token::Caret)) => {
                    self.tokens.next(); // consume ^
                    let sup = self.parse_group()?;
                    node = match node {
                        SemanticNode::Subscript { base, sub } => {
                            SemanticNode::SubSuperscript { base, sub, sup: Box::new(sup) }
                        }
                        _ => SemanticNode::Superscript { base: Box::new(node), sup: Box::new(sup) },
                    };
                }
                Some(Ok(Token::Underscore)) => {
                    self.tokens.next(); // consume _
                    let sub = self.parse_group()?;
                    node = match node {
                        SemanticNode::Superscript { base, sup } => {
                            SemanticNode::SubSuperscript { base, sub: Box::new(sub), sup }
                        }
                        _ => SemanticNode::Subscript { base: Box::new(node), sub: Box::new(sub) },
                    };
                }
                _ => break,
            }
        }

        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<SemanticNode> {
        let token_res = self.tokens.next().ok_or(RuTeXError::ParseError {
            message: "Unexpected end of input".to_string(),
            position: None,
        })?;

        let token = token_res.map_err(|_| RuTeXError::ParseError {
            message: "Invalid token".to_string(),
            position: None,
        })?;

        match token {
            Token::LBrace => {
                let node = self.parse_sequence()?;
                self.expect(Token::RBrace)?;
                Ok(node)
            }
            Token::Letter(c) => {
                let char = c.chars().next().unwrap();
                Ok(SemanticNode::Symbol {
                    glyph_key: GlyphKey {
                        char,
                        font_family: None,
                        style: FontStyle::Math,
                    },
                    role: SymbolRole::Ordinary,
                })
            }
            Token::Number(n) => Ok(SemanticNode::Text(n)),
            Token::Command(name) => self.parse_command(&name),
            Token::Operator(op) => {
                let char = op.chars().next().unwrap();
                Ok(SemanticNode::Symbol {
                    glyph_key: GlyphKey {
                        char,
                        font_family: None,
                        style: FontStyle::Normal,
                    },
                    role: self.infer_role(char),
                })
            }
            _ => Err(RuTeXError::ParseError {
                message: format!("Unexpected token: {:?}", token),
                position: None,
            }),
        }
    }

    fn parse_group(&mut self) -> Result<SemanticNode> {
        let token = self.tokens.peek().ok_or(RuTeXError::ParseError {
            message: "Expected group".to_string(),
            position: None,
        })?;

        match token {
            Ok(Token::LBrace) => {
                self.tokens.next();
                let node = self.parse_sequence()?;
                self.expect(Token::RBrace)?;
                Ok(node)
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_command(&mut self, name: &str) -> Result<SemanticNode> {
        match name {
            "frac" => {
                let num = self.parse_group()?;
                let den = self.parse_group()?;
                Ok(SemanticNode::Fraction {
                    num: Box::new(num),
                    den: Box::new(den),
                    line: rutex_types::LineStyle::Solid,
                })
            }
            "sqrt" => {
                // Check for optional degree [degree]
                let degree = if let Some(Ok(Token::LBracket)) = self.tokens.peek() {
                    self.tokens.next();
                    let d = self.parse_sequence()?;
                    self.expect(Token::RBracket)?;
                    Some(Box::new(d))
                } else {
                    None
                };
                let radicand = self.parse_group()?;
                Ok(SemanticNode::Radical {
                    degree,
                    radicand: Box::new(radicand),
                })
            }
            _ => {
                // Handle as a generic symbol for now
                // In a real implementation, we'd look this up in the macro system
                Ok(SemanticNode::Symbol {
                    glyph_key: GlyphKey {
                        char: '\\', // Placeholder
                        font_family: None,
                        style: FontStyle::Normal,
                    },
                    role: SymbolRole::Operator,
                })
            }
        }
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        let token_res = self.tokens.next().ok_or(RuTeXError::ParseError {
            message: format!("Expected {:?}, found end of input", expected),
            position: None,
        })?;

        let token = token_res.map_err(|_| RuTeXError::ParseError {
            message: "Invalid token".to_string(),
            position: None,
        })?;

        if token == expected {
            Ok(())
        } else {
            Err(RuTeXError::ParseError {
                message: format!("Expected {:?}, found {:?}", expected, token),
                position: None,
            })
        }
    }

    fn infer_role(&self, c: char) -> SymbolRole {
        match c {
            '+' | '-' | '*' | '/' => SymbolRole::Binary,
            '=' | '<' | '>' => SymbolRole::Relation,
            '(' | '[' | '{' => SymbolRole::Opening,
            ')' | ']' | '}' => SymbolRole::Closing,
            ',' | '.' | ':' | ';' => SymbolRole::Punctuation,
            _ => SymbolRole::Ordinary,
        }
    }
}
