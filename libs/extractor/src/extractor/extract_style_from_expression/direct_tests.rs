use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

fn styles(source: &str, name: Option<&str>) -> (Vec<ExtractStyleValue>, Vec<String>) {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("expression required")
    };
    let result = extract_style_from_expression(
        &ast,
        name,
        &mut statement.expression,
        0,
        &name.is_none().then(|| StyleSelector::from("hover")),
        LiteralHandling::ExpandResponsiveThemeToken,
    );
    let errors = result
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleProp::Unreadable { code, .. } => Some(code.clone()),
            _ => None,
        })
        .collect();
    (
        result
            .styles
            .into_iter()
            .flat_map(ExtractStyleProp::into_extract)
            .collect(),
        errors,
    )
}

#[rstest]
#[case("['red','blue'][-1]", &[])]
#[case("['red','blue'][1]", &["blue"])]
#[case("['red','blue'][0.5]", &[])]
#[case("['red',,'blue'][1]", &[])]
#[case("({a:'red',a:'blue'})['a']", &["blue"])]
#[case("({a:'red'})['missing']", &[])]
#[case("({a:'red',...rest,a:'green'})['a']", &["green"])]
#[case("null ?? 'red'", &["red"])]
fn literal_selection_when_the_selected_entry_is_known(
    #[case] source: &str,
    #[case] expected: &[&str],
) {
    let (actual, errors) = styles(source, Some("color"));
    let values: Vec<_> = actual
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value()),
            _ => None,
        })
        .collect();
    assert_eq!(values, expected, "{source}");
    assert_eq!(errors, Vec::<String>::new());
}

#[rstest]
#[case("['red',...rest][1]", "const rest = ['blue'];")]
#[case("({a:'red',...rest})['a']", "const rest = {a:'blue'};")]
fn literal_selection_when_a_spread_can_replace_the_entry_is_dynamic(
    #[case] source: &str,
    #[case] setup: &str,
) {
    let (actual, errors) = styles(source, Some("color"));
    let [ExtractStyleValue::Dynamic(style)] = actual.as_slice() else {
        panic!("dynamic style required: {actual:?}")
    };
    assert_eq!(evaluate(style.identifier(), setup), "blue");
    assert_eq!(errors, Vec::<String>::new());
}

#[test]
fn array_selection_when_the_key_is_runtime_keeps_known_prefix_and_spread() {
    let (actual, errors) = styles("['red',...rest][key]", Some("color"));
    assert_eq!(actual.len(), 2);
    assert!(actual.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.value() == "red")));
    assert!(actual.iter().any(|style| matches!(style,
        ExtractStyleValue::Dynamic(style) if style.identifier().contains("...rest") && style.identifier().contains("key"))));
    assert_eq!(errors, Vec::<String>::new());
}

#[rstest]
#[case("`${runtime}`")]
#[case("source[key]")]
fn selector_selection_when_the_whole_shape_is_opaque_is_unreadable(#[case] source: &str) {
    let (actual, errors) = styles(source, None);
    assert_eq!(actual, vec![]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains(source));
}

#[test]
fn sequence_when_written_into_a_dynamic_style_keeps_its_parentheses() {
    let (actual, errors) = styles("(first(), second())", Some("color"));
    let [ExtractStyleValue::Dynamic(style)] = actual.as_slice() else {
        panic!("dynamic style required: {actual:?}")
    };
    assert_eq!(
        evaluate(
            style.identifier(),
            "let trace=''; const first=()=>{trace+='1';}; const second=()=>{trace+='2';return trace;};"
        ),
        "12"
    );
    assert_eq!(errors, Vec::<String>::new());
}

fn evaluate(expression: &str, setup: &str) -> String {
    let script =
        format!("(()=>{{{setup} const style={{value:{expression}}}; return style.value;}})()");
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    value
        .to_string(&mut context)
        .unwrap_or_else(|error| panic!("{error}"))
        .to_std_string_escaped()
}
