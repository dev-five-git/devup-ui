use super::*;
use crate::extract_style::{ExtractStyleProperty, extract_static_style::ExtractStaticStyle};
use rstest::rstest;

#[rstest]
#[case("true", "red")]
#[case("false", "blue")]
#[serial_test::serial]
fn direct_condition_when_no_props_callback_exists_keeps_module_binding(
    #[case] on: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let imports = rustc_hash::FxHashMap::default();
    let values = crate::style_values::StyleValues::default();
    let inline_css = rustc_hash::FxHashMap::default();
    let bindings = StyledBindings {
        imports: &imports,
        values: &values,
        inline_css: &inline_css,
    };
    let expression = oxc_parser::Parser::new(
        &allocator,
        "on ? { color: 'red' } : { color: 'blue' }",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .map_err(|error| format!("failed to parse valid choice: {error:?}"))?;
    // When
    let mut styles = mixin(&ast, &expression, &bindings)
        .map_err(|_| "failed to lower written choice")?
        .ok_or("written choice produced no styles")?;
    let mut context = boa_engine::Context::default();
    let class = crate::gen_class_name::gen_class_names(&ast, &mut styles, None, None)
        .ok_or("conditional classes were missing")?;
    let code = format!(
        "const on = {on}; {}",
        crate::utils::expression_to_code(&class)
    );
    let selected = context
        .eval(boa_engine::Source::from_bytes(&code))
        .map_err(|error| format!("module condition failed to evaluate in scope: {error}"))?
        .to_string(&mut context)
        .map_err(|error| format!("selected class failed string conversion: {error}"))?
        .to_std_string_escaped();
    // Then
    let expected = ExtractStaticStyle::new("color", expected, 0, None)
        .extract(None)
        .to_string();
    assert_eq!(selected, expected);
    Ok(())
}

#[rstest]
#[case("(known) || { color: 'blue' }", false, "blue")]
#[case("(known) && { color: 'blue' }", true, "blue")]
#[case("(known) && unknownRules()", false, "")]
#[case("(known) ?? unknownRules()", false, "")]
#[serial_test::serial]
fn known_class_when_empty_or_nonempty_selects_rules_before_context_lowering(
    #[case] source: &str,
    #[case] truthy: bool,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let expression = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("failed to parse valid choice: {error:?}"))?;
    let Expression::LogicalExpression(logical) = &expression else {
        panic!("expected logical choice")
    };
    let known = if truthy {
        vec![crate::ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))]
    } else {
        vec![]
    };
    let imports = rustc_hash::FxHashMap::default();
    let values = crate::style_values::StyleValues::default();
    let inline_css =
        rustc_hash::FxHashMap::from_iter([(unwrap_syntax_only(&logical.left).span().start, known)]);
    let bindings = StyledBindings {
        imports: &imports,
        values: &values,
        inline_css: &inline_css,
    };
    // When
    let mut styles = mixin(&ast, &expression, &bindings)
        .map_err(|_| "failed to lower known class choice")?
        .ok_or("known class choice produced no styles")?;
    let class = crate::gen_class_name::gen_class_names(&ast, &mut styles, None, None)
        .map(|expression| crate::utils::expression_to_code(&expression));
    // Then
    match class {
        None => assert_eq!(expected, ""),
        Some(code) => {
            let selected = super::super::w27_props_rule_choices_tests::support::evaluate(
                crate::css_utils::rm_last_semi_colon(&code),
                "{}",
            );
            assert_eq!(
                selected,
                ExtractStaticStyle::new("color", expected, 0, None)
                    .extract(None)
                    .to_string()
            );
        }
    }
    Ok(())
}

#[test]
fn opaque_known_atom_when_nested_reports_error_instead_of_applying_global_class()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let expression = oxc_parser::Parser::new(
        &allocator,
        "`&:hover { ${known}; }`",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .map_err(|error| format!("failed to parse valid template: {error:?}"))?;
    let Expression::TemplateLiteral(template) = expression else {
        panic!("expected template")
    };
    let imports = rustc_hash::FxHashMap::default();
    let values = crate::style_values::StyleValues::default();
    let inline_css = rustc_hash::FxHashMap::from_iter([(
        template.expressions[0].span().start,
        vec![crate::ExtractStyleValue::Typography("body".into())],
    )]);
    let bindings = StyledBindings {
        imports: &imports,
        values: &values,
        inline_css: &inline_css,
    };
    // When
    let result = compose(&ast, &template, &bindings);
    // Then
    assert_eq!(result.styles.len(), 0);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].0, template.expressions[0].span().start);
    assert!(result.errors[0].1.contains("nested mixin"));
    Ok(())
}

#[test]
fn unknown_rule_when_reader_receives_it_fails_instead_of_silently_discarding_it() {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let imports = rustc_hash::FxHashMap::default();
    let values = crate::style_values::StyleValues::default();
    let inline_css = rustc_hash::FxHashMap::default();
    let bindings = StyledBindings {
        imports: &imports,
        values: &values,
        inline_css: &inline_css,
    };
    let mut rules = Expression::new_identifier(oxc_span::SPAN, "unknown", &ast);
    // When
    let result = MixinReader {
        ast: &ast,
        bindings: &bindings,
        params: None,
    }
    .read(&mut rules);
    // Then
    assert!(result.is_none());
}
