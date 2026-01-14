use rutex_renderer_svg::{SvgBackend, LayoutBackend};

#[test]
fn test_svg_backend_basic() {
    let mut backend = SvgBackend::new(100.0, 50.0);
    backend.render_rect(10.0, 10.0, 80.0, 30.0).unwrap();
    backend.render_text("Hello", 10.0, 40.0, 12.0, None).unwrap();
    
    let svg = backend.finish();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("width=\"100\""));
    assert!(svg.contains("height=\"50\""));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("Hello"));
}
