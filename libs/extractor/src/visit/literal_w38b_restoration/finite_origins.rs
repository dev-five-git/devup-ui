use super::*;

#[rstest]
#[case("css({color:true?'red':null})", "color-0-red--255", "string")]
#[case(
    "css({color:true?'red':'blue',bg:true?'white':'black'})",
    "background-0-white--255 color-0-red--255",
    "string"
)]
#[case("css({color:1===1?'red':null})", "color-0-red--255", "conditional")]
#[case(
    "css({color:1===1?'red':'blue',bg:1===1?'white':'black'})",
    "background-0-white--255 color-0-red--255",
    "template"
)]
#[case(
    "css`style-order:${1===1?2:3};color:red`",
    "color-0-red--2",
    "template"
)]
#[serial]
fn standalone_when_closed_origin_survives_capture_restores_final_source_span(
    #[case] call: &str,
    #[case] expected: &str,
    #[case] shape: &str,
) {
    // Given: the ordinary visitor reads an actual standalone imported css call.
    let source = format!("import {{css}} from '@devup-ui/react';{call};");
    let allocator = Allocator::default();
    let mut parsed = oxc_parser::Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("standalone call required")
    };
    let span = statement.expression.span();
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    visitor.source = Some(&source);
    css::debug::set_debug(true);
    // When: normal program visiting supplies composition, capture and finite provenance.
    visitor.visit_program(&mut parsed.program);
    css::debug::set_debug(false);
    let Statement::ExpressionStatement(statement) = parsed
        .program
        .body
        .last()
        .unwrap_or_else(|| panic!("compiled expression"))
    else {
        panic!("compiled expression required")
    };
    let value = &statement.expression;
    // Then: the actual origin gate, final shape, primitive result and call span agree.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert!(
        visitor.style_values.at(span).is_some(),
        "{call}: {}",
        readable_code(value)
    );
    assert_eq!(evaluated(value, ""), serde_json::json!([expected, []]));
    assert_eq!(root(value), shape);
    assert_eq!(value.span(), span);
}

#[rstest]
#[case(true, "red")]
#[case(false, "blue")]
#[serial]
fn captured_origin_when_revisited_preserves_identifier_and_one_producer_read(
    #[case] active: bool,
    #[case] color: &str,
) {
    // Given: a real saved finite class supplies capture_as through capture_shape.
    let source = "import {css} from '@devup-ui/react';const saved=css({color:state.active?'red':'blue'});saved;";
    let allocator = Allocator::default();
    let mut parsed = oxc_parser::Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    visitor.source = Some(source);
    css::debug::set_debug(true);
    visitor.visit_program(&mut parsed.program);
    css::debug::set_debug(false);
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    let Statement::ExpressionStatement(statement) = parsed
        .program
        .body
        .pop()
        .unwrap_or_else(|| panic!("saved read"))
    else {
        panic!("saved read required")
    };
    let mut value = statement.unbox().expression;
    let span = value.span();
    assert!(visitor.style_values.finite(&value).is_some());
    let mut captures = Vec::new();
    visitor.capture_shape(&mut value, &mut captures);
    assert_eq!(captures.len(), 1);
    assert!(visitor.style_values.at(span).is_some());
    assert_eq!(root(&value), "identifier");
    let before = readable_code(&value);
    // When: expression visiting receives the actual captured-origin identifier.
    visitor.visit_expression(&mut value);
    // Then: the default restoration action is a no-op, not a blanket span setter.
    assert_eq!(root(&value), "identifier");
    assert_eq!(value.span(), span);
    assert_eq!(readable_code(&value), before);
    assert!(visitor.style_values.at(span).is_some());
    let program = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let value = call_with_values(&visitor.ast, captures, value);
    let setup = format!(
        "const state={{get active(){{trace.push('producer');return {active};}}}};{program}"
    );
    assert_eq!(
        evaluated(&value, &setup),
        serde_json::json!([format!("color-0-{color}--255"), ["producer"]])
    );
}
