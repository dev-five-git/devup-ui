use crate::{ExtractOption, extract, extract_style::extract_style_value::ExtractStyleValue};
use boa_engine::{Context, Source};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

mod execution;
mod infinity;

#[derive(Clone, Copy)]
enum Surface {
    Box,
    Css,
    Styled,
    Responsive,
}

#[derive(Clone, Copy)]
enum Expected {
    Static(&'static str),
    Dynamic,
}

#[rstest]
#[case::effectful(("opacity", "`0.${(trace.push('value'),5)}`", Expected::Dynamic))]
#[case::unknown(("opacity", "`0.${value}`", Expected::Dynamic))]
#[case::pure(("opacity", "`0.${5}`", Expected::Static(".5")))]
#[case::nested_pure(("opacity", "`0.${`${5}`}`", Expected::Static(".5")))]
#[case::nested_effectful(("opacity", "`0.${`${(trace.push('value'),5)}`}`", Expected::Dynamic))]
#[case::string_spelling(("opacity", "`0.${'05'}`", Expected::Static(".05")))]
#[case::cooked(("opacity", r"`\x30.${5}`", Expected::Static(".5")))]
#[case::text_static(("color", "`r${'ed'}`", Expected::Static("red")))]
#[case::text_dynamic(("color", "`r${value}`", Expected::Dynamic))]
#[serial]
fn template_values_preserve_interpolations_when_extracted(
    #[case] sample: (&str, &str, Expected),
    #[values(Surface::Box, Surface::Css, Surface::Styled, Surface::Responsive)] surface: Surface,
    #[values(false, true)] single_css: bool,
) {
    // Given: runtime arguments cannot be mistaken for build-time constants.
    reset_class_map();
    reset_file_map();
    let (property, expression, expected) = sample;
    let statement = match surface {
        Surface::Box => format!("return <Box {property}={{{expression}}} />;"),
        Surface::Css => format!("return css({{ {property}: {expression} }});"),
        Surface::Styled => format!("return styled.div({{ {property}: {expression} }});"),
        Surface::Responsive => {
            format!("return <Box {property}={{['inherit',null,{expression}]}} />;")
        }
    };
    let source = format!(
        "import {{ Box, css, styled }} from '@devup-ui/react';\nfunction render(value, trace) {{ {statement} }}"
    );
    // When
    let result = extract(
        "numeric-template.tsx",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    // Then: css has no element to own runtime variables; the other surfaces do.
    match (surface, expected, result) {
        (Surface::Css, Expected::Dynamic, Err(error)) => {
            let message = error.to_string();
            assert!(message.starts_with("numeric-template.tsx:2:"), "{message}");
            assert!(message.contains("`css()` cannot use"), "{message}");
            assert!(message.contains("at build time:"), "{message}");
        }
        (Surface::Css, Expected::Dynamic, Ok(output)) => {
            panic!("runtime css unexpectedly folded: {output:?}")
        }
        (_, _, Err(error)) => panic!("template extraction failed: {error}"),
        (_, expected, Ok(output)) => {
            let level = match surface {
                Surface::Responsive => 2,
                Surface::Box | Surface::Css | Surface::Styled => 0,
            };
            assert_eq!(output.styles.len(), if level == 2 { 2 } else { 1 });
            for style in &output.styles {
                match style {
                    ExtractStyleValue::Static(style) if style.level == 0 && level == 2 => {
                        assert_eq!(
                            (style.property(), style.value(), style.selector()),
                            (property, "inherit", None)
                        );
                    }
                    ExtractStyleValue::Static(style) => {
                        let Expected::Static(value) = expected else {
                            panic!("runtime template became static: {style:?}")
                        };
                        assert_eq!(
                            (
                                style.property(),
                                style.level(),
                                style.value(),
                                style.selector()
                            ),
                            (property, level, value, None)
                        );
                    }
                    ExtractStyleValue::Dynamic(style) => {
                        assert!(
                            matches!(expected, Expected::Dynamic),
                            "pure template became dynamic: {style:?}"
                        );
                        assert_eq!(
                            (style.property(), style.level(), style.selector()),
                            (property, level, None)
                        );
                        match surface {
                            Surface::Box | Surface::Responsive => {
                                let arena = Allocator::default();
                                let input = Parser::new(&arena, expression, SourceType::mjs())
                                    .parse_expression()
                                    .unwrap_or_else(|error| {
                                        panic!("invalid input expression: {error:?}")
                                    });
                                let retained =
                                    Parser::new(&arena, style.identifier(), SourceType::mjs())
                                        .parse_expression()
                                        .unwrap_or_else(|error| {
                                            panic!("invalid dynamic expression: {error:?}")
                                        });
                                let evaluate = |expression| {
                                    let script = format!(
                                        "const trace=[]; const value=5; JSON.stringify([{expression},trace]);"
                                    );
                                    Context::default()
                                        .eval(Source::from_bytes(&script))
                                        .unwrap_or_else(|error| {
                                            panic!("template evaluation failed: {error}")
                                        })
                                        .as_string()
                                        .unwrap_or_else(|| panic!("template must return JSON text"))
                                        .to_std_string_escaped()
                                };
                                assert_eq!(
                                    evaluate(crate::utils::readable_code(&retained)),
                                    evaluate(crate::utils::readable_code(&input))
                                );
                            }
                            Surface::Styled => assert_ne!(style.identifier(), ""),
                            Surface::Css => unreachable!("runtime css must fail extraction"),
                        }
                    }
                    other => panic!("unexpected template style: {other:?}"),
                }
            }
        }
    }
}

#[rstest]
#[case::complete("`0.${5}`", Some(0.5))]
#[case::unknown("`0.${value}`", None)]
#[case::effectful("`0.${(trace.push('value'),5)}`", None)]
#[case::nested_pure("`0.${`${5}`}`", Some(0.5))]
#[case::nested_unknown("`0.${`${value}`}`", None)]
#[case::nested_effectful("`0.${`${(trace.push('value'),5)}`}`", None)]
#[case::string_spelling("`0.${'05'}`", Some(0.05))]
#[case::cooked(r"`\x30.${5}`", Some(0.5))]
#[case::parenthesized("(`0.${5}`)", Some(0.5))]
#[case::parenthesized_string("`0.${('05')}`", Some(0.05))]
#[case::unary_substitution("`${-5}`", Some(-5.0))]
#[case::nonnumeric_template("`r${'ed'}`", None)]
#[case::escaped_rust_infinity_alias(r"`\x69nf`", None)]
fn numeric_literal_probe_reads_complete_template_when_static(
    #[case] source: &str,
    #[case] expected: Option<f64>,
) {
    // Given: parser-created quasis preserve the distinction between raw and cooked.
    let arena = Allocator::default();
    let expression = Parser::new(&arena, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid template fixture: {error:?}"));
    // When
    let actual = crate::utils::get_number_by_literal_expression(&expression);
    // Then
    match (actual, expected) {
        (Some(actual), Some(expected)) => assert!(
            (actual - expected).abs() <= f64::EPSILON,
            "{source}: {actual} != {expected}"
        ),
        (None, None) => {}
        other => panic!("wrong numeric template resolution for {source}: {other:?}"),
    }
}

#[rstest]
#[case::complete("`0.${5}`", Some("0.5"))]
#[case::unknown("`0.${value}`", None)]
#[case::effectful("`0.${(trace.push('value'),5)}`", None)]
#[case::nested_pure("`0.${`${5}`}`", Some("0.5"))]
#[case::nested_unknown("`0.${`${value}`}`", None)]
#[case::nested_effectful("`0.${`${(trace.push('value'),5)}`}`", None)]
#[case::string_spelling("`0.${'05'}`", Some("0.05"))]
#[case::nested_string_spelling("`0.${`${'05'}`}`", Some("0.05"))]
#[case::cooked(r"`\x30.${5}`", Some("0.5"))]
#[case::text_static("`r${'ed'}`", Some("red"))]
#[case::text_dynamic("`r${value}`", None)]
#[case::parenthesized("(`0.${'05'}`)", Some("0.05"))]
#[case::parenthesized_string("`0.${('05')}`", Some("0.05"))]
#[case::true_substitution("`x${true}`", Some("xtrue"))]
#[case::false_substitution("`x${false}`", Some("xfalse"))]
#[case::negative_substitution("`${-5}`", Some("-5"))]
#[case::unary_string("`${+'05'}`", Some("5"))]
#[case::unary_nested_template("`${-`${5}`}`", Some("-5"))]
#[case::unsupported_unary("`${!5}`", None)]
#[case::escaped_rust_infinity_alias(r#"`"value:${+`\x69nf`}"`"#, None)]
fn string_literal_probe_preserves_template_text_when_static(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    // Given
    let arena = Allocator::default();
    let expression = Parser::new(&arena, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid template fixture: {error:?}"));
    // When
    let actual = crate::utils::get_string_by_literal_expression(&expression);
    // Then
    assert_eq!(actual.as_deref(), expected, "{source}");
}

#[rstest]
#[case("`0.${5}`")]
#[case("`0.${(trace.push('value'),5)}`")]
fn template_probe_returns_unknown_when_generated_quasi_has_no_cooked_text(#[case] source: &str) {
    // Given: generated template nodes can omit cooked text even though the source parsed.
    let arena = Allocator::default();
    let mut expression = Parser::new(&arena, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid template fixture: {error:?}"));
    let oxc_ast::ast::Expression::TemplateLiteral(template) = &mut expression else {
        panic!("fixture must be a template");
    };
    template.quasis[0].value.cooked = None;
    // When
    let actual = crate::utils::get_string_by_literal_expression(&expression);
    // Then: unreadable text is never replaced by raw quasis or a fabricated number.
    assert_eq!(actual, None);
}
