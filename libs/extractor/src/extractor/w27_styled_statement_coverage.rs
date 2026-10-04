use super::super::*;
use rstest::rstest;

#[rstest]
#[case("p => p.on ? {opacity: 0.4} : p.rules")]
#[case("function(p) { return p.on ? {opacity: 0.4} : p.rules; }")]
#[serial_test::serial]
fn unknown_written_callback_branch_when_top_level_reports_original_interpolation(
    #[case] callback: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{ styled }} from '@devup-ui/react';\nexport const Choice = styled.div`${{{callback}}};`;"
    );
    let column = source
        .rsplit('\n')
        .next()
        .ok_or("missing fixture line")?
        .find(callback)
        .ok_or("missing fixture callback")?
        + 1;
    // When
    let error = crate::extract(
        "unknown-callback.tsx",
        &source,
        crate::ExtractOption::default(),
    )
    .err()
    .ok_or("unknown written callback branch compiled")?
    .to_string();
    // Then
    assert!(
        error.starts_with(&format!("unknown-callback.tsx:2:{column}:")),
        "{error}"
    );
    assert!(
        error.contains("p.rules") && error.contains("object literal"),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case("p => p.on && p.external")]
#[case("function(p) { return p.on && p.external; }")]
#[serial_test::serial]
fn opaque_statement_callback_when_top_level_is_called_with_props_and_false_becomes_empty(
    #[case] callback: &str,
    #[values(false, true)] on: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = format!("styled.div`${{{callback}}};`");
    let mut expression = Parser::new(&allocator, &source, SourceType::tsx())
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
    // Then: opaque class callbacks legitimately reach the statement fallback.
    assert_eq!(output.errors, vec![]);
    let definition = output.definition.ok_or("missing callback definition")?;
    assert_eq!(
        definition.reads,
        Reads {
            names: vec!["on".into(), "external".into()],
            whole: false
        }
    );
    assert_eq!(definition.classes.len(), 1);
    let class = definition
        .classes
        .first()
        .ok_or("expected callback class expression")?;
    let code = expression_to_code(class);
    let selected = super::super::w27_props_rule_choices_tests::support::evaluate(
        crate::css_utils::rm_last_semi_colon(&code),
        &format!("{{on: {on}, external: 'external-class'}}"),
    );
    assert_eq!(selected, if on { "external-class" } else { "" });
    Ok(())
}

#[test]
fn uncurried_styled_when_second_argument_is_callback_stays_object_only()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = "styled('div', p => ({opacity: p.opacity}))";
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
    assert!(output.definition.is_none());
    assert_eq!(
        output.errors,
        vec![(
            0,
            build_time_error("styled", &readable_code(&expression), STYLED_FACTORY)
        )]
    );
    assert_eq!(
        expression_to_code(&output.expression),
        expression_to_code(&expression)
    );
    Ok(())
}
