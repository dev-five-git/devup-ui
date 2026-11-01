use super::*;

#[test]
fn nested_array_spreads_when_composed_preserve_later_rules_and_class_order() {
    // Given: statically known nested array spreads contain both rules and classes.
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let expression = oxc_parser::Parser::new(
        &arena,
        "compose([{color:'red'},...[['first',{padding:1}],...[{color:'blue'},'last']]])",
        oxc_span::SourceType::mjs(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::CallExpression(call) = expression else {
        panic!("expected composition call")
    };
    // When: the real collector flattens statically known spreads before merging rules.
    let StyleArguments { classes, rules } = style_arguments(&ast, &call.arguments)
        .unwrap_or_else(|| panic!("expected composable nested spread"));
    let classes = classes
        .iter()
        .map(readable_code)
        .collect::<Vec<_>>()
        .join(",");
    let script = format!("JSON.stringify([{},[{}]]);", readable_code(&rules), classes);
    let actual = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON observation"))
        .to_std_string_escaped();
    // Then: the later color wins without losing the earlier independent declaration or either class.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("{error}")),
        serde_json::json!([{"padding":1,"color":"blue"},["first","last"]])
    );
}
