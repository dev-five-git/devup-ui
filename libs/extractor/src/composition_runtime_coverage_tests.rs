use super::*;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::BTreeMap;

#[test]
fn overlap_when_runtime_key_map_is_unresolved_is_conservative() {
    // Given: disjoint atoms would not compete, but runtime maps have unknown keys.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let color = ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle::new(
        "color", "red", 0, None,
    )));
    let width = ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle::new(
        "width", "10px", 0, None,
    )));
    let enum_prop = ExtractStyleProp::Enum {
        condition: Expression::new_identifier(SPAN, "variant", &builder),
        map: BTreeMap::from([("wide".to_string(), vec![width.clone_in(&allocator)])]),
    };
    let member = ExtractStyleProp::MemberExpression {
        expression: Expression::new_identifier(SPAN, "variant", &builder),
        map: BTreeMap::from([("wide".to_string(), Box::new(width.clone_in(&allocator)))]),
    };
    // When / Then: test both operand positions and the genuinely disjoint control.
    assert!(overlaps(
        std::slice::from_ref(&color),
        std::slice::from_ref(&enum_prop)
    ));
    assert!(overlaps(
        std::slice::from_ref(&enum_prop),
        std::slice::from_ref(&color)
    ));
    assert!(overlaps(
        std::slice::from_ref(&color),
        std::slice::from_ref(&member)
    ));
    assert!(overlaps(
        std::slice::from_ref(&member),
        std::slice::from_ref(&color)
    ));
    assert!(!overlaps(&[color], &[width]));
}

#[rstest]
#[case("css(known, ...unknown)", true)]
#[case("css(known, {color:'blue'})", false)]
#[serial]
fn known_composition_when_argument_is_spread_rejects_before_expression_conversion(
    #[case] expression: &str,
    #[case] spread: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let source = format!("const known = ''; {expression};");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let symbol = scoping
        .get_root_binding("known".into())
        .ok_or("missing known binding")?;
    let mut visitor = DevupVisitor::new(&allocator, "spread.tsx", "@devup-ui/react", vec![], None);
    visitor.style_values = crate::style_values::StyleValues::new(scoping);
    visitor.style_values.insert(
        symbol,
        crate::style_values::StyleValue::Class(
            "known".into(),
            Some(vec![ExtractStyleValue::Static(ExtractStaticStyle::new(
                "color", "red", 0, None,
            ))]),
        ),
    );
    let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("expected expression");
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("expected call");
    };
    // When
    let result = visitor.compose_known_styles(call, false);
    // Then
    if spread {
        assert!(result.is_none());
        assert_eq!(visitor.styles, FxHashSet::default());
        assert_eq!(visitor.css_styles, None);
        assert_eq!(visitor.inline_css_styles, FxHashMap::default());
    } else {
        let result = result.ok_or("known composition was rejected")?;
        let expected = vec![ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "blue", 0, None,
        ))];
        assert!(matches!(result, Expression::StringLiteral(_)));
        assert_eq!(visitor.styles, FxHashSet::from_iter(expected.clone()));
        assert_eq!(
            visitor.css_styles,
            Some((call.span.start, expected.clone()))
        );
        assert_eq!(
            visitor.inline_css_styles.get(&call.span.start),
            Some(&expected)
        );
    }
    Ok(())
}

#[test]
#[serial]
fn inline_tagged_choice_when_reused_retains_atoms_and_choice_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: tagged composition retains a conditional atom, not an opaque class.
    let allocator = Allocator::default();
    let source = "const known = ''; css`${flag ? known : null}`;";
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let symbol = scoping
        .get_root_binding("known".into())
        .ok_or("missing known binding")?;
    let red = ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None));
    let mut visitor = DevupVisitor::new(&allocator, "inline.tsx", "@devup-ui/react", vec![], None);
    visitor.style_values = crate::style_values::StyleValues::new(scoping);
    visitor.style_values.insert(
        symbol,
        crate::style_values::StyleValue::Class("known".into(), Some(vec![red.clone()])),
    );
    let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("expected expression");
    };
    let Expression::TaggedTemplateExpression(tag) = &statement.expression else {
        panic!("expected tag");
    };
    let compiled = visitor
        .compose_template(tag, false)
        .ok_or("tag was rejected")?;
    // When: a conditional side cannot erase the nested choice to one class.
    let side = visitor.known_side(&compiled, Text::Classes);
    // Then
    assert!(side.is_none());
    assert_eq!(compiled.span().start, tag.span.start);
    assert_eq!(visitor.styles, FxHashSet::from_iter([red.clone()]));
    assert_eq!(visitor.css_styles, None);
    let parts = visitor
        .inline_css_parts
        .get(&tag.span.start)
        .ok_or("missing inline parts")?;
    assert!(
        matches!(&parts[..], [KnownPart::Conditional {test, consequent, alternate}]
        if readable_code(test) == "flag" && alternate.is_empty()
        && matches!(&consequent[..], [KnownStyles::Known(values)] if values == &[red]))
    );
    Ok(())
}
