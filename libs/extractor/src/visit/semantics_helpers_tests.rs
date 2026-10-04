use super::*;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use std::borrow::Cow;

#[rstest]
#[case("false ?? 'external'", None)]
#[case("false || 'external'", Some("external"))]
#[case("null ?? 'external'", Some("external"))]
fn composition_when_an_empty_left_value_has_different_logical_meanings(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    let visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let mut parts = Vec::new();
    assert_eq!(
        visitor.known_parts(&statement.expression, &mut parts, Text::Classes),
        Some(())
    );
    let classes: Vec<_> = parts
        .iter()
        .filter_map(|part| match part {
            KnownPart::Class(Expression::StringLiteral(value)) => Some(value.value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(classes, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn known_spread_pruning_when_a_key_is_computed_keeps_it_and_dom_props() {
    let allocator = Allocator::default();
    let mut parsed = Parser::new(
        &allocator,
        "({color:'red',id:'keep',[key]:'computed',...rest})",
        SourceType::ts(),
    )
    .parse();
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("expression required")
    };
    let Expression::ObjectExpression(object) =
        crate::utils::unwrap_syntax_only_mut(&mut statement.expression)
    else {
        panic!("object required")
    };
    let mut written = FxHashSet::from_iter([Cow::Borrowed("color")]);
    jsx_order::prune_shadowed(object, &mut written);
    assert_eq!(object.properties.len(), 3);
    assert!(object.properties.iter().any(|property| matches!(property,
        ObjectPropertyKind::ObjectProperty(property) if property.computed)));
    assert!(object.properties.iter().any(|property| matches!(property,
        ObjectPropertyKind::ObjectProperty(property) if property.key.static_name().as_deref() == Some("id"))));
}

#[test]
fn spread_attribute_when_an_element_has_both_kinds_selects_only_the_spread() {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "<Box id='keep' {...rest}/>", SourceType::tsx()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("element required")
    };
    let Expression::JSXElement(element) = &statement.expression else {
        panic!("element required")
    };
    let spreads: Vec<_> = element
        .opening_element
        .attributes
        .iter()
        .filter_map(jsx_order::spread_attribute)
        .collect();
    assert_eq!(spreads.len(), 1);
    assert!(matches!(&spreads[0].argument, Expression::Identifier(value) if value.name == "rest"));
}

#[rstest]
#[case(
    "typography",
    "heading",
    crate::extract_style::extract_static_style::ThemeTokenResolution::CssVariable
)]
#[case(
    "padding",
    "$space",
    crate::extract_style::extract_static_style::ThemeTokenResolution::FirstValue
)]
fn fallback_binding_when_one_variable_cannot_represent_a_shape_is_located(
    #[case] property: &str,
    #[case] value: &str,
    #[case] resolution: crate::extract_style::extract_static_style::ThemeTokenResolution,
) {
    let allocator = Allocator::default();
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let style = crate::extract_style::extract_static_style::ExtractStaticStyle::new(
        property, value, 0, None,
    )
    .with_theme_token_resolution(resolution);
    let bound = visitor.bind_overridden(
        "Box",
        vec![Overridden {
            key: property.to_string(),
            offset: 12,
            spreads: 1,
            styles: vec![ExtractStyleProp::Static(ExtractStyleValue::Static(style))],
        }],
        &["__devupSpread0".to_string()],
    );
    assert_eq!(bound.len(), 0);
    assert_eq!(visitor.errors.len(), 1);
    assert_eq!(visitor.errors[0].0, 12);
    assert!(visitor.errors[0].1.contains(property));
}
