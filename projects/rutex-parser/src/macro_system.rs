use im::HashMap;
use rutex_types::{MathStyle, Result, RuTeXError};
use crate::lexer::Token;

#[derive(Clone, Debug)]
pub struct MacroDefinition {
    pub name: String,
    pub args_count: usize,
    pub body: Vec<Token>,
}

#[derive(Clone, Debug)]
pub struct ParseContext {
    pub macros: HashMap<String, MacroDefinition>,
    pub math_style: MathStyle,
    pub environment_stack: Vec<String>,
}

impl Default for ParseContext {
    fn default() -> Self {
        Self {
            macros: HashMap::new(),
            math_style: MathStyle::Display,
            environment_stack: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseEffect {
    MacroExpansion {
        name: String,
        args: Vec<Vec<Token>>,
    },
    StyleChange(MathStyle),
    BeginEnv(String),
    EndEnv,
}

impl ParseContext {
    pub fn apply_effect(&self, effect: ParseEffect) -> Result<Self> {
        let mut new_ctx = self.clone();
        match effect {
            ParseEffect::MacroExpansion { .. } => {
                // Macro expansion doesn't change the context itself in this simple model,
                // but in a more complex one it might (e.g. \def).
            }
            ParseEffect::StyleChange(style) => {
                new_ctx.math_style = style;
            }
            ParseEffect::BeginEnv(env) => {
                new_ctx.environment_stack.push(env);
            }
            ParseEffect::EndEnv => {
                if new_ctx.environment_stack.pop().is_none() {
                    return Err(RuTeXError::ParseError {
                        message: "Unexpected \\end".to_string(),
                        position: None,
                    });
                }
            }
        }
        Ok(new_ctx)
    }
}
