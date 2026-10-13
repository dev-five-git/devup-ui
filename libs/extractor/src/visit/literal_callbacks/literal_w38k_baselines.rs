use super::compile;
use super::literal_w38k_support::{PLAIN, declarations, fixture, red};
use oxc_ast::ast::{Expression, Statement};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{throw 1;}")]
#[case("p=>{throw 1;return;}")]
#[case("p=>{if(p.stop)throw 1;return;}")]
#[case("p=>{while(true){}return;}")]
#[case("p=>{for(;;){}return;}")]
#[case("p=>{do{return;}while(p.stop);return;}")]
#[case("p=>{try{return;}finally{throw 1;}return;}")]
#[case("p=>{try{throw 1;}finally{return;}return;}")]
#[case("p=>{label:{return;}return;}")]
#[case("p=>{return;throw 1;}")]
#[serial]
fn unsupported_bare_route_when_control_is_explicit_keeps_baseline_error(#[case] callback: &str) {
    // Given: authored unsupported bodies, including an unreachable explicit throw.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), PLAIN);
    let input = format!("`${{{callback}}}`;");
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, &input, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("callback fixture")
    };
    let Expression::TemplateLiteral(template) = &statement.expression else {
        panic!("interpolation fixture")
    };
    let original = crate::utils::unwrap_syntax_only(&template.expressions[0]);
    let expected = crate::style_order::invalid_order(&crate::utils::readable_code(original));
    // When: public extraction encounters the new bare route; no body is executed.
    let error = compile(&source)
        .err()
        .unwrap_or_else(|| panic!("unsupported body compiled"));
    // Then: the whole located original invalid-order receipt remains unchanged.
    assert_eq!(error, format!("a.tsx:1:115: {expected}"));
}

#[rstest]
#[case("p=>{throw 1;return 2;}")]
#[case("p=>{if(p.stop)throw 1;return 2;}")]
#[case("p=>{while(true){}return 2;}")]
#[case("p=>{for(;;){}return 2;}")]
#[case("p=>{do{return;}while(p.stop);return 2;}")]
#[case("p=>{try{return;}finally{throw 1;}return 2;}")]
#[case("p=>{try{throw 1;}finally{return;}return 2;}")]
#[case("p=>{label:{return;}return 2;}")]
#[serial]
fn old_valued_route_when_prefix_is_unsupported_preserves_acceptance(#[case] callback: &str) {
    // Given: the same unsupported prefixes followed by the old terminal valued return.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), PLAIN);
    // When: public extraction follows the existing valued route without executing loops.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    // Then: its complete candidate declaration vector remains present, not filtered out.
    assert_eq!(declarations(&output), red(&[None, Some(2)]));
}
