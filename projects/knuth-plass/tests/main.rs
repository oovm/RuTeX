use rutex_knuth_plass::{Item, KnuthPlass};

#[test]
fn test_simple_line_break() {
    let items = vec![
        Item::Box { width: 40.0, debug_info: Some("word1".into()) },
        Item::Glue { width: 10.0, stretch: 5.0, shrink: 2.0 },
        Item::Box { width: 40.0, debug_info: Some("word2".into()) },
        Item::Glue { width: 10.0, stretch: 5.0, shrink: 2.0 },
        Item::Box { width: 40.0, debug_info: Some("word3".into()) },
    ];

    // Try to fit in 100 width. 40+10+40 = 90 (good). 40+10+40+10+40 = 140 (too much).
    let kp = KnuthPlass::new(vec![100.0], 10.0);
    let breaks = kp.find_breaks(&items);

    // It should break after "word2" or just fit everything if tolerance allows.
    // In this case, 140 vs 100 is too much.
    // 40+10+40 = 90. 90 is close to 100.
    assert!(!breaks.is_empty());
    println!("Breaks: {:?}", breaks);
}

#[test]
fn test_exact_fit() {
    let items = vec![
        Item::Box { width: 50.0, debug_info: None },
        Item::Glue { width: 10.0, stretch: 0.0, shrink: 0.0 },
        Item::Box { width: 40.0, debug_info: None },
    ];

    let kp = KnuthPlass::new(vec![100.0], 1.0);
    let breaks = kp.find_breaks(&items);
    
    // Should fit perfectly in one line, so breaks might be empty (only end)
    // or contain the last index.
    println!("Exact fit breaks: {:?}", breaks);
}

#[test]
fn test_multiple_lines() {
    let items = vec![
        Item::Box { width: 30.0, debug_info: Some("1".into()) },
        Item::Glue { width: 10.0, stretch: 2.0, shrink: 1.0 },
        Item::Box { width: 30.0, debug_info: Some("2".into()) },
        Item::Glue { width: 10.0, stretch: 2.0, shrink: 1.0 },
        Item::Box { width: 30.0, debug_info: Some("3".into()) },
        Item::Glue { width: 10.0, stretch: 2.0, shrink: 1.0 },
        Item::Box { width: 30.0, debug_info: Some("4".into()) },
        Item::Glue { width: 10.0, stretch: 2.0, shrink: 1.0 },
        Item::Box { width: 30.0, debug_info: Some("5".into()) },
    ];

    // Width 50. Each box is 30. 30+10+30 = 70 (too wide).
    // So it should break after each box or every two boxes if glue allows.
    // With 50, it must break after each box.
    let kp = KnuthPlass::new(vec![50.0], 10.0);
    let breaks = kp.find_breaks(&items);
    println!("Multiple lines breaks: {:?}", breaks);
    assert!(breaks.len() >= 2);
}
