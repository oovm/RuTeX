use serde::{Serialize, Deserialize};
pub use rutex_types::{RuTeXError, Result};

/// 架构核心：定义不可变、可序列化的语义树，作为各阶段间唯一接口。
#[derive(Debug, Serialize, Deserialize)]
pub struct MathSemanticTree {
    pub root: SemanticNode,
    // 包含所有宏展开后的源映射信息，用于调试和富交互。
}

/// 语义节点：描述“是什么”，而非“怎么画”。
#[derive(Debug, Serialize, Deserialize)]
pub enum SemanticNode {
    Sequence(Vec<SemanticNode>),
    HorizontalBox { 
        content: Vec<SemanticNode>, 
        spacing: SpacingRule 
    },
    VerticalBox { 
        content: Vec<SemanticNode>, 
        alignment: Alignment 
    },
    Fraction { 
        num: Box<SemanticNode>, 
        den: Box<SemanticNode>, 
        line: LineStyle 
    },
    Radical { 
        degree: Option<Box<SemanticNode>>, 
        radicand: Box<SemanticNode> 
    },
    Symbol { 
        glyph_key: String, 
        role: SymbolRole 
    }, // 关键：符号与角色分离
    // ... 矩阵、上下标等
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SpacingRule {
    Default,
    Tight,
    Loose,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Alignment {
    Center,
    Left,
    Right,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SymbolRole {
    Ordinary,
    Operator,
    Binary,
    Relation,
    Open,
    Close,
    Punctuation,
}

/// 解析与宏系统
pub mod parser {
    use super::*;
    use std::collections::HashMap;

    pub struct ParseContext {
        pub macros: HashMap<String, String>, // 简化版宏定义
        pub math_style: MathStyle,
        pub environment_stack: Vec<String>,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub enum MathStyle {
        Display,
        Text,
        Script,
        ScriptScript,
    }

    pub enum ParseEffect {
        MacroExpansion(String, Vec<String>),
        StyleChange(MathStyle),
        BeginEnv(String),
        EndEnv,
    }
}

/// 排版引擎
pub mod layout {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum LayoutItem {
        Box { width: f64, height: f64, depth: f64, node: Option<SemanticNode> },
        Glue { width: f64, stretch: f64, shrink: f64 },
        Penalty { cost: f64, width: f64, flagged: bool },
    }

    pub fn knuth_plass_line_break(_items: &[LayoutItem], _line_widths: &[f64]) -> Vec<usize> {
        // 动态规划寻找最优断点，最小化“不良度”。
        vec![]
    }
}

/// 字体与度量
pub mod font {
    use super::*;
    use std::sync::Arc;
    use dashmap::DashMap;

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
            // 异步加载，全局缓存
            self.cache.get(glyph_key)
                .map(|m| GlyphMetrics { ..*m })
                .ok_or_else(|| RuTeXError::FontError { 
                    glyph: glyph_key.to_string(), 
                    message: "Glyph not found in metrics system".to_string() 
                })
        }
    }
}

/// 渲染后端
pub mod renderer {
    use super::*;

    pub trait LayoutBackend {
        fn render_horizontal_box(&mut self, content: &[SemanticNode], spacing: SpacingRule) -> Result<()>;
        fn render_radical(&mut self, degree: Option<&SemanticNode>, radicand: &SemanticNode) -> Result<()>;
    }
}
