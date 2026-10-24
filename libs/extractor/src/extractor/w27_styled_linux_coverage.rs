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
    let mut rules = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let before = expression_to_code(&rules);
    // When
    let mut evaluations =
        crate::extractor::styled_record_evaluation::Evaluations::new(&ast, before.clone());
    let result =
        props::lower(&mut evaluations, &mut rules, None).map_err(|error| format!("{error:?}"))?;
    // Then
    assert!(result.is_none());
    assert_eq!(evaluations.bindings.len(), 0);
    assert_eq!(expression_to_code(&rules), before);
    Ok(())
}

#[test]
#[serial_test::serial]
fn direct_props_lowerer_when_object_is_written_preserves_props_value()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source =
        "import {styled} from '@devup-ui/react';const Choice=styled.div(p=>({opacity:p.opacity}));";
    // When
    let output = crate::extract(
        "opacity.tsx",
        source,
        crate::ExtractOption {
            single_css: true,
            ..crate::ExtractOption::default()
        },
    )?;
    // Then
    assert_eq!(
        crate::styled_source_tests::captured_render::execute(
            ("opacity.tsx", None),
            &output,
            "const element=render({target:Choice,props:{ opacity: 0.4 }});JSON.stringify({ opacity: values(element,'opacity')[0] });",
        ),
        serde_json::json!({"opacity":0.4})
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
        (&[], None, &[]),
        None,
        StyledContext {
            filename: "linux.tsx",
            jsx: &mut None,
            capability: None,
        },
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
        (&[], None, &[]),
        None,
        StyledContext {
            filename: "linux.tsx",
            jsx: &mut None,
            capability: None,
        },
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
