use rutex_layout::{LayoutEngine, LayoutNode, MathStyle, SemanticNode, GlyphKey, FontStyle, Fixed};
use rutex_font::{FontMetricsSystem, FontLoader, GlyphMetrics, Result, RuTeXError};
use rutex_types::{SymbolRole, LineStyle};
use async_trait::async_trait;
use std::sync::Arc;

struct MockLoader;
#[async_trait]
impl FontLoader for MockLoader {
    async fn load_font_data(&self, _family: &str) -> Result<Arc<Vec<u8>>> {
        Ok(Arc::new(vec![]))
    }
}

#[tokio::test]
async fn test_basic_layout() {
    let loader = Arc::new(MockLoader);
    let font_system = FontMetricsSystem::new(loader);
    
    let key = GlyphKey {
        char: 'a',
        font_family: None,
        style: FontStyle::Normal,
    };
    
    font_system.insert_metrics(key.clone(), GlyphMetrics {
        width: Fixed::from_f64(10.0),
        height: Fixed::from_f64(8.0),
        depth: Fixed::from_f64(2.0),
        italic_correction: Fixed::ZERO,
    });

    let engine = LayoutEngine::new(font_system);
    let node = SemanticNode::Symbol {
        glyph_key: key,
        role: SymbolRole::Ordinary,
    };
    
    let layout = engine.layout_node(&node, MathStyle::Text).await.unwrap();
    
    assert_eq!(layout.width(), Fixed::from_f64(10.0));
    assert_eq!(layout.height(), Fixed::from_f64(8.0));
    assert_eq!(layout.depth(), Fixed::from_f64(2.0));
}

#[tokio::test]
async fn test_sequence_layout() {
    let loader = Arc::new(MockLoader);
    let font_system = FontMetricsSystem::new(loader);
    
    let key_a = GlyphKey { char: 'a', font_family: None, style: FontStyle::Normal };
    let key_b = GlyphKey { char: 'b', font_family: None, style: FontStyle::Normal };
    
    font_system.insert_metrics(key_a.clone(), GlyphMetrics {
        width: Fixed::from_f64(10.0),
        height: Fixed::from_f64(8.0),
        depth: Fixed::from_f64(2.0),
        italic_correction: Fixed::ZERO,
    });
    font_system.insert_metrics(key_b.clone(), GlyphMetrics {
        width: Fixed::from_f64(12.0),
        height: Fixed::from_f64(9.0),
        depth: Fixed::from_f64(1.0),
        italic_correction: Fixed::ZERO,
    });

    let engine = LayoutEngine::new(font_system);
    let node = SemanticNode::Sequence(vec![
        SemanticNode::Symbol { glyph_key: key_a, role: SymbolRole::Ordinary },
        SemanticNode::Symbol { glyph_key: key_b, role: SymbolRole::Ordinary },
    ]);
    
    let layout = engine.layout_node(&node, MathStyle::Text).await.unwrap();
    
    assert_eq!(layout.width(), Fixed::from_f64(22.0));
    assert_eq!(layout.height(), Fixed::from_f64(9.0));
    assert_eq!(layout.depth(), Fixed::from_f64(2.0));
}

#[tokio::test]
async fn test_fraction_layout() {
    let loader = Arc::new(MockLoader);
    let font_system = FontMetricsSystem::new(loader);
    
    let key_a = GlyphKey { char: 'a', font_family: None, style: FontStyle::Normal };
    let key_b = GlyphKey { char: 'b', font_family: None, style: FontStyle::Normal };

    font_system.insert_metrics(key_a.clone(), GlyphMetrics {
        width: Fixed::from_f64(10.0), height: Fixed::from_f64(10.0), depth: Fixed::ZERO, italic_correction: Fixed::ZERO
    });
    font_system.insert_metrics(key_b.clone(), GlyphMetrics {
        width: Fixed::from_f64(10.0), height: Fixed::from_f64(10.0), depth: Fixed::ZERO, italic_correction: Fixed::ZERO
    });

    let engine = LayoutEngine::new(font_system);
    let num = SemanticNode::Symbol { glyph_key: key_a, role: SymbolRole::Ordinary };
    let den = SemanticNode::Symbol { glyph_key: key_b, role: SymbolRole::Ordinary };
    let frac = SemanticNode::Fraction {
        num: Box::new(num),
        den: Box::new(den),
        line: LineStyle::Solid,
    };
    
    let layout = engine.layout_node(&frac, MathStyle::Display).await.unwrap();
    assert!(layout.width() >= Fixed::from_f64(10.0));
}

#[tokio::test]
async fn test_paragraph_layout() {
    let loader = Arc::new(MockLoader);
    let font_system = FontMetricsSystem::new(loader);
    
    let key_a = GlyphKey { char: 'a', font_family: None, style: FontStyle::Normal };
    
    font_system.insert_metrics(key_a.clone(), GlyphMetrics {
        width: Fixed::from_f64(10.0),
        height: Fixed::from_f64(8.0),
        depth: Fixed::from_f64(2.0),
        italic_correction: Fixed::ZERO,
    });

    let engine = LayoutEngine::new(font_system);
    
    // 10 symbols 'a', each 10.0 wide, plus glue (3.0 wide).
    // Total width roughly: 10 * 10 + 9 * 3 = 127.0.
    // Width limit 50.0. Should break into at least 3 lines.
    let mut content = Vec::new();
    for _ in 0..10 {
        content.push(SemanticNode::Symbol { glyph_key: key_a.clone(), role: SymbolRole::Ordinary });
    }
    
    let node = SemanticNode::Paragraph {
        content,
        width: Fixed::from_f64(50.0),
    };
    
    let layout = engine.layout_node(&node, MathStyle::Text).await.unwrap();
    
    if let LayoutNode::VBox(v) = layout {
         // Check that we have multiple lines
         assert!(v.children.len() >= 3);
         let num_lines = v.children.len();
         for (i, line) in v.children.iter().enumerate() {
             if let LayoutNode::HBox(h) = line {
                 if i < num_lines - 1 {
                     // Non-last lines should be justified to the target width
                     assert_eq!(h.width, Fixed::from_f64(50.0));
                 } else {
                     // The last line should have its natural width (less than or equal to target width)
                     assert!(h.width <= Fixed::from_f64(50.0));
                 }
             }
         }
     } else {
        panic!("Expected VBox for paragraph layout");
    }
}
