use super::{DevupVisitor, KnownPart, Text};
use crate::ExtractStyleValue;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::style_values::{StyleValue, StyleValues};
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("opaque", false)]
#[case("external", true)]
#[serial]
fn template_composition_when_only_opaque_classes_remain_preserves_their_form(
    #[case] name: &str,
    #[case] literal: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let source = format!("css`${{known}}${{{name}}}`");
    let mut visitor =
        DevupVisitor::new(&allocator, "coverage.tsx", "@devup-ui/react", vec![], None);
    let combined = format!("const known = ''; const external = ''; {source};");
    let parsed = Parser::new(&allocator, &combined, SourceType::tsx()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    visitor.style_values = StyleValues::new(scoping);
    let known = visitor
        .style_values
        .root_symbol("known")
        .ok_or("missing known binding")?;
    let external = visitor
        .style_values
        .root_symbol("external")
        .ok_or("missing external binding")?;
    visitor
        .style_values
        .insert(known, StyleValue::Class(String::new(), Some(vec![])));
    visitor
        .style_values
        .insert(external, StyleValue::Class("external".into(), None));
    let oxc_ast::ast::Statement::ExpressionStatement(statement) =
        parsed.program.body.last().ok_or("missing expression")?
    else {
        return Err("expected expression fixture".into());
    };
    let Expression::TaggedTemplateExpression(tag) = &statement.expression else {
        return Err("expected tag".into());
    };
    // When
    let result = visitor
        .compose_template(tag, false)
        .ok_or("composition was not recognized")?;
    // Then
    assert_eq!(matches!(result, Expression::StringLiteral(_)), literal);
    assert_eq!(
        crate::utils::readable_code(&result),
        if literal { "\"external\"" } else { "opaque" }
    );
    assert_eq!(visitor.styles.len(), 0);
    Ok(())
}

#[rstest]
#[case("css(known, { color: 'blue' })")]
#[case("css(flag ? known : null, { color: 'blue' })")]
#[serial]
fn known_call_when_later_rules_replace_all_choices_records_exact_inline_metadata(
    #[case] call: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let source = format!("const known = ''; {call};");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let known = scoping
        .get_root_binding("known".into())
        .ok_or("missing known binding")?;
    let mut visitor =
        DevupVisitor::new(&allocator, "coverage.tsx", "@devup-ui/react", vec![], None);
    visitor.style_values = StyleValues::new(scoping);
    visitor.style_values.insert(
        known,
        StyleValue::Class(
            "known".into(),
            Some(vec![ExtractStyleValue::Static(ExtractStaticStyle::new(
                "color", "red", 0, None,
            ))]),
        ),
    );
    let oxc_ast::ast::Statement::ExpressionStatement(statement) =
        parsed.program.body.last().ok_or("missing expression")?
    else {
        return Err("expected call fixture".into());
    };
    let Expression::CallExpression(call) = &statement.expression else {
        return Err("expected call".into());
    };
    // When
    let result = visitor
        .compose_known_styles(call, false)
        .ok_or("known call was unreadable")?;
    // Then
    let expected = vec![ExtractStyleValue::Static(ExtractStaticStyle::new(
        "color", "blue", 0, None,
    ))];
    assert_eq!(
        visitor.css_styles,
        Some((call.span.start, expected.clone()))
    );
    assert_eq!(
        visitor.inline_css_styles.get(&call.span.start),
        Some(&expected)
    );
    assert!(
        matches!(result, Expression::StringLiteral(literal) if literal.span.start == call.span.start)
    );
    Ok(())
}

#[test]
#[serial]
fn known_array_when_it_contains_holes_skips_them_without_losing_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        "const known = ''; [, known];",
        SourceType::tsx(),
    )
    .parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let known = scoping
        .get_root_binding("known".into())
        .ok_or("missing known binding")?;
    let style = ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None));
    let mut visitor =
        DevupVisitor::new(&allocator, "coverage.tsx", "@devup-ui/react", vec![], None);
    visitor.style_values = StyleValues::new(scoping);
    visitor.style_values.insert(
        known,
        StyleValue::Class("known".into(), Some(vec![style.clone()])),
    );
    let oxc_ast::ast::Statement::ExpressionStatement(statement) =
        parsed.program.body.last().ok_or("missing expression")?
    else {
        return Err("expected array fixture".into());
    };
    let mut parts = Vec::new();
    // When
    visitor
        .known_parts(&statement.expression, &mut parts, Text::Classes)
        .ok_or("array was unreadable")?;
    // Then
    assert_eq!(parts.len(), 1);
    assert!(
        matches!(&parts[0], KnownPart::Styles(styles) if matches!(&styles[..], [super::KnownStyles::Known(values)] if values == &[style]))
    );
    Ok(())
}
