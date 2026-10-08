use super::*;
use rstest::rstest;

#[path = "w27_styled_statement_coverage.rs"]
mod statements;

#[rstest]
#[case("unknownRules")]
#[case("[{}]")]
#[case("false")]
fn direct_props_lowerer_when_rules_are_unsupported_rejects_without_mutation(
    #[case] source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let callback = Parser::new(&allocator, "p => null", SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let callback = match callback {
        Expression::ArrowFunctionExpression(callback) => Some(callback),
        _ => None,
    }
    .ok_or("expected callback fixture")?;
    let mut rules = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let before = expression_to_code(&rules);
    // When
    let result = props_rules(&ast, &mut rules, &callback.params);
    // Then
    assert_eq!(result, None);
    assert_eq!(expression_to_code(&rules), before);
    Ok(())
}

#[test]
fn direct_props_lowerer_when_object_is_written_preserves_props_value()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let callback = Parser::new(&allocator, "p => null", SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let callback = match callback {
        Expression::ArrowFunctionExpression(callback) => Some(callback),
        _ => None,
    }
    .ok_or("expected callback fixture")?;
    let mut rules = Parser::new(&allocator, "({opacity: p.opacity})", SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    // When
    let result = props_rules(&ast, &mut rules, &callback.params);
    // Then
    assert_eq!(result, Some(()));
    let code = expression_to_code(&rules);
    assert_eq!(
        super::w27_props_rule_choices_tests::support::evaluate(
            &format!(
                "JSON.stringify(({}))",
                crate::css_utils::rm_last_semi_colon(&code)
            ),
            "{opacity: 0.4}",
        ),
        r#"{"opacity":0.4}"#
    );
    Ok(())
}

#[rstest]
#[case("styled.div(...[{opacity: p => p.$opacity}])", true)]
#[case("styled.div(...unknownRules)", false)]
#[serial_test::serial]
fn raw_styled_spread_when_received_by_extractor_keeps_reads_or_composition_error(
    #[case] source: &str,
    #[case] reads_props: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the public visitor may inline known spreads before this helper sees them.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let imports = FxHashMap::default();
    let values = crate::style_values::StyleValues::default();
    let inline_css = FxHashMap::default();
    // When
    let output = extract_style_from_styled(
        &ast,
        &mut expression,
        Naming::default(),
        StyledBindings {
            imports: &imports,
            values: &values,
            inline_css: &inline_css,
        },
        &[],
        None,
        None,
    );
    // Then
    if reads_props {
        let definition = output.definition.ok_or("missing spread definition")?;
        assert_eq!(
            definition.reads,
            Reads {
                names: vec!["$opacity".into()],
                whole: false
            }
        );
        assert!(definition.takes("$opacity"));
        assert_eq!(output.errors.len(), 1);
        let (offset, error) = output
            .errors
            .first()
            .ok_or("expected spread callback error")?;
        assert_eq!(
            *offset,
            u32::try_from(source.find("p =>").ok_or("missing callback location")?)?
        );
        assert!(
            error.contains("p.$opacity") && error.contains("object literal"),
            "{error}"
        );
    } else {
        assert_eq!(output.errors.len(), 1);
        let (offset, error) = output
            .errors
            .first()
            .ok_or("expected spread composition error")?;
        assert_eq!(*offset, 0);
        assert!(
            error.contains("unknownRules") && error.contains("at build time"),
            "{error}"
        );
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn raw_styled_spread_when_array_contains_written_objects_extracts_exact_value()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = Parser::new(
        &allocator,
        "styled.div(...[{opacity: 0.6}])",
        SourceType::tsx(),
    )
    .parse_expression()
    .map_err(|error| format!("{error:?}"))?;
    let imports = FxHashMap::default();
    let values = crate::style_values::StyleValues::default();
    let inline_css = FxHashMap::default();
    // When
    let output = extract_style_from_styled(
        &ast,
        &mut expression,
        Naming::default(),
        StyledBindings {
            imports: &imports,
            values: &values,
            inline_css: &inline_css,
        },
        &[],
        None,
        None,
    );
    // Then
    assert_eq!(output.errors, vec![]);
    let styles = output
        .result
        .styles
        .iter()
        .flat_map(ExtractStyleProp::extract)
        .collect::<Vec<_>>();
    assert_eq!(
        super::w27_props_rule_choices_tests::support::declarations(&styles),
        [("opacity".into(), ".6".into())]
    );
    Ok(())
}
