use rutex_parser::parse;
use rutex_types::{SemanticNode, MathSemanticTree};

#[test]
fn test_basic_arithmetic() {
    let input = "a + b = c";
    let tree = parse(input).unwrap();
    
    if let SemanticNode::Sequence(nodes) = tree.root {
        assert_eq!(nodes.len(), 5);
    } else {
        panic!("Expected sequence");
    }
}

#[test]
fn test_fraction() {
    let input = "\\frac{1}{2}";
    let tree = parse(input).unwrap();
    
    match tree.root {
        SemanticNode::Fraction { .. } => {},
        _ => panic!("Expected fraction, got {:?}", tree.root),
    }
}

#[test]
fn test_sub_sup() {
    let input = "x^2 + y_1";
    let tree = parse(input).unwrap();
    
    if let SemanticNode::Sequence(nodes) = tree.root {
        assert_eq!(nodes.len(), 3);
        match &nodes[0] {
            SemanticNode::Superscript { .. } => {},
            _ => panic!("Expected superscript"),
        }
        match &nodes[2] {
            SemanticNode::Subscript { .. } => {},
            _ => panic!("Expected subscript"),
        }
    } else {
        panic!("Expected sequence");
    }
}

#[test]
fn test_sqrt() {
    let input = "\\sqrt{x} + \\sqrt[3]{y}";
    let tree = parse(input).unwrap();
    
    if let SemanticNode::Sequence(nodes) = tree.root {
        assert_eq!(nodes.len(), 3);
        match &nodes[0] {
            SemanticNode::Radical { degree, .. } => assert!(degree.is_none()),
            _ => panic!("Expected radical"),
        }
        match &nodes[2] {
            SemanticNode::Radical { degree, .. } => assert!(degree.is_some()),
            _ => panic!("Expected radical with degree"),
        }
    }
}
