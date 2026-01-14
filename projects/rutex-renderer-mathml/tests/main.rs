use rutex_renderer_mathml::{MathmlBackend, LayoutBackend, Result};

#[test]
fn test_mathml_basic() -> Result<()> {
    let mut backend = MathmlBackend::new();
    backend.render_rect(0.0, 0.0, 100.0, 50.0)?;
    backend.render_text("123", 10.0, 20.0, 12.0, None)?;
    
    let mml = backend.finish();
    assert!(mml.contains(r#"<math"#));
    assert!(mml.contains(r#"<mspace width="100px" height="50px""#));
    assert!(mml.contains(r#"<mtext style="font-size: 12px;">123</mtext>"#));
    Ok(())
}

#[test]
fn test_mathml_groups() -> Result<()> {
    let mut backend = MathmlBackend::new();
    backend.start_group(Some("translate(10, 20)"))?;
    backend.render_text("x", 0.0, 0.0, 10.0, None)?;
    backend.end_group()?;
    
    let mml = backend.finish();
    assert!(mml.contains(r#"<mpadded voffset="20px" loffset="10px">"#));
    assert!(mml.contains(r#"<mi style="font-size: 10px;">x</mi>"#));
    Ok(())
}
