use crate::{
    ExtractStyleValue,
    assignment_lowering::{AlternateOrder, Lowering},
    assignment_test_support::{evaluate, extracted, jsx_js},
    extract_style::style_property::StyleProperty,
    extractor::extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    utils::expression_to_code,
};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

#[path = "typography_order_capture_tests.rs"]
mod order_capture;

#[rstest]
#[case("['heading']", "typo-heading")]
#[case("['body']", "typo-body")]
#[serial]
fn array_packet_keeps_base_in_both_class_projections_when_order_is_alternate(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given: the real typography extraction IR and both packet class slots.
    let _theme = super::TypographyTheme::register();
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut program = Parser::new(&allocator, source, SourceType::ts())
        .parse()
        .program;
    let Statement::ExpressionStatement(statement) = &mut program.body[0] else {
        panic!("array fixture");
    };
    let mut styles = extract_style_from_expression(
        &ast,
        Some("typography"),
        &mut statement.expression,
        0,
        &None,
        LiteralHandling::KeepSingleClass,
    )
    .styles;
    let lowering = Lowering {
        ast: &ast,
        order: Some(1),
        filename: None,
        alternate_order: Some(AlternateOrder { value: Some(2) }),
    };
    // When: the production array lowering creates its actual primary/alternate packet.
    let packet = lowering.lower(&statement.expression, &mut styles);
    let actual = evaluate(&format!(
        "const packet={};JSON.stringify([packet[0],packet[3],packet[1],packet[2]]);",
        expression_to_code(&packet).trim().trim_end_matches(';')
    ));
    // Then: neither order loses the class-only base or changes the authored array.
    let preset = expected.trim_start_matches("typo-");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("packet JSON: {error}")),
        serde_json::json!([expected, expected, {}, [preset]])
    );
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn responsive_array_keeps_selected_roles_when_classname_and_styleorder_share_capture(
    #[case] choice: bool,
) {
    // Given: className and dynamic styleOrder share a real JSX assignment capture.
    let _theme = super::TypographyTheme::register();
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box className={{state.name}} styleOrder={{state.order?1:2}} typography={{state.flag?['heading','body']:['body','heading']}}/>}}const state={{name:'external',order:{choice},flag:{choice}}};const node=render(state);"
    );
    let output = extracted(&source);
    let records = output
        .styles
        .iter()
        .filter_map(|value| match (value, value.extract(None)?) {
            (ExtractStyleValue::Static(style), StyleProperty::ClassName(class)) => {
                Some((class, style.value.clone(), style.level, style.style_order))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let records =
        serde_json::to_string(&records).unwrap_or_else(|error| panic!("records JSON: {error}"));
    // When: emitted classes are joined to their actual order/breakpoint records.
    let actual = evaluate(&format!(
        "{}const records={records};const classes=String(node.className).split(/\\s+/).filter(Boolean);JSON.stringify([classes.filter(name=>name==='external'||name.startsWith('typo-')).sort(),classes.filter(name=>name!=='external'&&!name.startsWith('typo-')).map(name=>records.find(([key])=>key===name).slice(1))]);",
        jsx_js(&output.code)
    ));
    // Then: the independent class and selected order preserve base and later roles.
    let (base, later, order) = if choice {
        ("typo-heading", "body", 1)
    } else {
        ("typo-body", "heading", 2)
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("selected JSON: {error}")),
        serde_json::json!([["external", base], [[later, 1, order]]])
    );
}
