use super::super::StyledBindings;
use super::*;

#[rstest::rstest]
#[case("unknown() && {color:'red'}", false)]
#[case("(p=>p.theme.brand()) && {color:'red'}", true)]
fn direct_schedule_when_controller_is_unreadable_returns_the_original_error(
    #[case] source: &str,
    #[case] theme: bool,
) {
    // Given
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut expression = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let scope = expression.clone_in_with_semantic_ids(&arena);
    let values = crate::style_values::StyleValues::default();
    let inline_css = rustc_hash::FxHashMap::default();
    let bindings = StyledBindings {
        imports: &|_| None,
        values: &values,
        inline_css: &inline_css,
    };
    let mut evaluations = Evaluations::new(&ast, source.into());
    // When
    let Err(error) = lower_direct(&mut evaluations, &mut expression, (&bindings, &scope)) else {
        panic!("invalid controller cannot be scheduled")
    };
    // Then
    assert_eq!(error.1.contains("theme"), theme, "{}", error.1);
    assert_eq!(evaluations.bindings.len(), 0);
    assert_eq!(readable_code(&expression), readable_code(&scope));
}

#[test]
fn callback_record_when_later_spread_is_unknown_rolls_back_only_its_own_captures() {
    // Given
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let source = "({color:p=>p.color,...unknown})";
    let expression = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut evaluations = Evaluations::new(&ast, source.into());
    evaluations.bindings.push((
        "earlier".into(),
        Expression::new_boolean_literal(SPAN, true, &ast),
    ));
    // When
    let prepared =
        crate::extractor::styled_record::prepare_callbacks(&mut evaluations, &expression);
    // Then
    assert!(prepared.is_none());
    assert_eq!(evaluations.bindings.len(), 1);
    assert_eq!(evaluations.bindings[0].0, "earlier");
    assert!(matches!(&evaluations.bindings[0].1, Expression::BooleanLiteral(value) if value.value));
}

#[rstest::rstest]
#[case("styled.div`${{color:p=>p.color}};`", "{color:'red'}", serde_json::json!(["red"]))]
#[case("styled.div(p=>({color:p.on?(p.input.inner?p.color:null):null}))", "{on:true,input:{ inner: true },color:'red'}", serde_json::json!(["red"]))]
#[case("styled.div(p=>({color:p.on?(p.input.inner?p.color:null):null}))", "{on:false,input:{get inner(){throw Error('inactive')}}}", serde_json::json!([]))]
#[case("styled.div(p=>(p.on||false)&&{color:'red'})", "{on:false}", serde_json::json!([]))]
#[case("styled.div(p=>(p.on||false)&&{color:'red'})", "{on:true}", serde_json::json!(["red"]))]
#[case("styled.div(p=>({[p.input.key]:p.color,get unused(){throw Error('getter must not run')}})&&{color:'blue'})", "{input:{key:'color'},color:'red'}", serde_json::json!(["blue"]))]
#[serial_test::serial]
fn rendered_rules_when_nested_choices_or_template_property_callbacks_select_keep_authored_values(
    #[case] rules: &str,
    #[case] props: &str,
    #[case] expected: serde_json::Value,
) {
    // Given
    let source = format!("import {{styled}} from '@devup-ui/react';const Choice={rules};");
    let output = crate::extract(
        "choice.tsx",
        &source,
        crate::ExtractOption {
            single_css: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // When
    let observed = crate::styled_source_tests::captured_render::execute(
        ("choice.tsx", None),
        &output,
        &format!(
            "const element=render({{target:Choice,props:{props}}});JSON.stringify(values(element,'color'));"
        ),
    );
    // Then
    assert_eq!(observed, expected);
}
