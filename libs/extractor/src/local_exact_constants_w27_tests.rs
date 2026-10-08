use std::collections::HashMap;

use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[path = "local_shape_composition_w27_tests.rs"]
mod composition;

#[path = "local_shape_hygiene_w27_tests.rs"]
mod hygiene;

fn emotion_option() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    }
}

#[rstest]
#[case(
    "const red = 'red'; export function f() { const inner = { color: red }; return css(inner); }",
    "color",
    "red"
)]
#[case(
    "export function f() { const red = 'red'; const inner = { color: red }; return css(inner); }",
    "color",
    "red"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 }; return css(inner); }",
    "padding",
    "8px"
)]
#[case(
    "const red = 'blue'; export function f() { const red = 'red'; const inner = { color: red }; return css(inner); }",
    "color",
    "red"
)]
#[case(
    "export function a() { const red = 'blue'; } export function f() { const red = 'red'; const inner = { color: red }; return css(inner); }",
    "color",
    "red"
)]
#[case(
    "export function f() { const gap = 2; const doubled = gap * 2; const inner = { padding: Math.max(doubled, 8) }; return css(inner); }",
    "padding",
    "8px"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: `${gap * 4}px` }; return css(inner); }",
    "padding",
    "8px"
)]
#[case(
    "export function f() { const gap = 2; const inner = [{ padding: gap * 4 }]; return css(inner); }",
    "padding",
    "8px"
)]
#[case(
    "export function f() { const gap = 2; console.log(gap); const inner = { padding: -gap }; return css(inner); }",
    "padding",
    "-2px"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 } as const; return css(inner); }",
    "padding",
    "8px"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 }; return <div css={inner} />; }",
    "padding",
    "8px"
)]
#[serial]
fn exact_rules_when_dependencies_are_scoped_constants(
    #[case] body: &str,
    #[case] property: &str,
    #[case] value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("import {{ css }} from '@emotion/react';\n{body}");
    // When
    let output = extract("local-exact.tsx", &code, emotion_option())?;
    // Then
    let styles: Vec<(&str, &str)> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => (style.property.as_str(), style.value.as_str()),
            other => panic!("expected static style, got {other:?}"),
        })
        .collect();
    assert_eq!(styles, vec![(property, value)]);
    assert!(!output.code.contains("css(inner)"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case(
    "const red = 'red'; export function f(red) { const inner = { color: red }; return css(inner); }"
)]
#[case(
    "const red = 'red'; export function f() { const inner = { color: red }; const red = 'blue'; return css(inner); }"
)]
#[case(
    "export function f() { const inner = { padding: gap * 4 }; const gap = 2; return css(inner); }"
)]
#[case(
    "export function f() { const gap = gap; const inner = { padding: gap }; return css(inner); }"
)]
#[case(
    "export function f() { let gap = 2; const inner = { padding: gap * 4 }; return css(inner); }"
)]
#[case(
    "export function f() { const gap = 2; gap = 3; const inner = { padding: gap * 4 }; return css(inner); }"
)]
#[case("export function f(gap) { const inner = { padding: gap * 4 }; return css(inner); }")]
#[case(
    "export function f() { const gap = getGap(); const inner = { padding: gap * 4 }; return css(inner); }"
)]
#[case("export function f(Math) { const inner = { padding: Math.max(2, 8) }; return css(inner); }")]
#[case(
    "export function f() { const Math = { max: () => 8 }; const inner = { padding: Math.max(2, 8) }; return css(inner); }"
)]
#[case(
    "export function f() { const gap = { size: 2 }; mutate(gap); const inner = { padding: gap.size * 4 }; return css(inner); }"
)]
#[case(
    "export function f() { const gap = 2; return css(inner); const inner = { padding: gap * 4 }; }"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 }; console.log(inner); return css(inner); }"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 }; inner.padding = 12; return css(inner); }"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 }; const alias = inner; return css(inner); }"
)]
#[case(
    "export function f() { const gap = 2; const inner = { padding: gap * 4 }; return <div css={inner} title={inner} />; }"
)]
#[case(
    "export function f(undefined) { const gap = undefined; const inner = { padding: gap }; return css(inner); }"
)]
#[case(
    "export function f() { const gap = Math.random(); const inner = { padding: gap }; return css(inner); }"
)]
#[serial]
fn located_error_when_rules_are_unknown_changed_or_escaped(#[case] body: &str) {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("import {{ css }} from '@emotion/react';\n{body}");
    // When
    let Err(error) = extract("local-exact.tsx", &code, emotion_option()) else {
        panic!("expected located error");
    };
    let error = error.to_string();
    // Then
    assert!(error.starts_with("local-exact.tsx:2:"), "{error}");
    assert!(error.contains("inner"), "{error}");
}

#[test]
#[serial]
fn runtime_value_when_written_directly_on_an_element_keeps_css_variable()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = "import { css } from '@emotion/react';\nexport function f(gap) { return <div css={{ padding: gap * 4 }} />; }";
    // When
    let output = extract("local-exact.tsx", code, emotion_option())?;
    // Then
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
    );
    assert!(output.code.contains("gap*4"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case(
    "const inner = { color: 'red', m: 2 }; return <Box {...inner} />;",
    "red"
)]
#[case(
    "const inner = { color: 'red' }; return <Box color='blue' {...inner} />;",
    "red"
)]
#[case(
    "const inner = { color: 'red' }; return <Box {...inner} color='blue' />;",
    "blue"
)]
#[case(
    "const inner = { color: 'red', padding: 2 }; const C = styled('div', inner); return <C />;",
    "red"
)]
#[case(
    "const inner = { color: 'red' }; return <div css={[{ color: 'blue' }, inner]} />;",
    "red"
)]
#[case(
    "const inner = { color: 'red' }; return <div css={[inner, { color: 'blue' }]} />;",
    "blue"
)]
#[serial]
fn literal_shape_when_styles_are_composed_obeys_later_wins(
    #[case] body: &str,
    #[case] color: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{ Box, styled }} from '@devup-ui/react';\nexport function f() {{ {body} }}"
    );
    // When
    let output = extract("local-exact.tsx", &code, emotion_option())?;
    // Then
    let colors: Vec<&str> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "color" => {
                Some(style.value.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(colors, vec![color]);
    Ok(())
}

#[rstest]
#[case(
    "const inner = { color: props.getColor(), _hover: { color: props.getHover() } }; return <Box {...inner} />;"
)]
#[case(
    "const inner = { color: props.getColor(), _hover: { color: props.getHover() } }; return <div css={inner} />;"
)]
#[case(
    "const inner = { color: props.getColor(), _hover: { color: props.getHover() } }; const C = styled('div', inner); return <C />;"
)]
#[case(
    "const inner = { color: props.getColor(), _hover: { color: props.getHover() } }; return <div css={[{ color: 'blue' }, inner]} />;"
)]
#[case(
    "const inner = { color: [props.getColor(), 'red'], _hover: { color: props.getHover() } }; return <Box {...inner} />;"
)]
#[serial]
fn captured_runtime_shape_when_expanded_reads_original_data_paths(
    #[case] body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{ Box, styled }} from '@devup-ui/react';\nexport function f(props) {{ {body} }}"
    );
    // When
    let output = extract("local-exact.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        output.code.matches("props.getColor()").count(),
        1,
        "{}",
        output.code
    );
    assert_eq!(
        output.code.matches("props.getHover()").count(),
        1,
        "{}",
        output.code
    );
    assert!(output.code.contains("inner[`color`]"), "{}", output.code);
    assert!(
        output.code.contains("inner[`_hover`][`color`]"),
        "{}",
        output.code
    );
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
            .count(),
        2
    );
    Ok(())
}

#[rstest]
#[case(
    "const inner = { color: props.c }; return <Box color='blue' {...inner} />;",
    true
)]
#[case(
    "const inner = { color: props.c }; return <Box {...inner} color='blue' />;",
    false
)]
#[case(
    "const inner = { color: props.c }; return <div css={[{ color: 'blue' }, inner]} />;",
    true
)]
#[case(
    "const inner = { color: props.c }; return <div css={[inner, { color: 'blue' }]} />;",
    false
)]
#[serial]
fn dynamic_shape_when_overridden_keeps_only_winning_color(
    #[case] body: &str,
    #[case] dynamic: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code =
        format!("import {{ Box }} from '@devup-ui/react';\nexport function f(props) {{ {body} }}");
    // When
    let output = extract("local-exact.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
            .count(),
        usize::from(dynamic),
        "{output:?}"
    );
    let colors: Vec<&str> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "color" => {
                Some(style.value.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(colors, if dynamic { vec![] } else { vec!["blue"] });
    Ok(())
}

#[rstest]
#[case("const inner = { color: props.c }; inner.color = 'blue'; return <Box {...inner} />;")]
#[case("const inner = { color: props.c }; mutate(inner); return <Box {...inner} />;")]
#[case("const inner = { color: props.c }; const alias = inner; return <div css={inner} />;")]
#[case("const inner = { color: props.c }; console.log(inner.color); return <div css={inner} />;")]
#[case("const inner = { color: props.c }; return <Box {...inner} data-inner={inner} />;")]
#[case("const inner = { color: props.c }; inner = {}; return <Box {...inner} />;")]
#[case("const inner = { color: red }; const red = 'red'; return <Box {...inner} />;")]
#[case("const inner = { color: inner }; return <Box {...inner} />;")]
#[case("return <Box {...inner} />; const inner = { color: props.c };")]
#[serial]
fn local_runtime_shape_when_changed_or_escaped_is_a_located_error(#[case] body: &str) {
    // Given
    reset_class_map();
    reset_file_map();
    let code =
        format!("import {{ Box }} from '@devup-ui/react';\nexport function f(props) {{ {body} }}");
    // When
    let Err(error) = extract("local-exact.tsx", &code, emotion_option()) else {
        panic!("expected located error");
    };
    let error = error.to_string();
    // Then
    assert!(error.starts_with("local-exact.tsx:2:"), "{error}");
    assert!(error.contains("inner"), "{error}");
}

#[rstest]
#[case("const inner = { ...props }; return <Box {...inner} />;")]
#[case("const inner = getProps(); return <Box {...inner} />;")]
#[serial]
fn unknown_object_when_spread_remains_unexpanded(
    #[case] body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code =
        format!("import {{ Box }} from '@devup-ui/react';\nexport function f(props) {{ {body} }}");
    // When
    let output = extract("local-exact.tsx", &code, emotion_option())?;
    // Then
    assert!(output.code.contains("...inner"), "{}", output.code);
    assert_eq!(output.styles.len(), 0);
    Ok(())
}

#[test]
#[serial]
fn probe_when_all_three_exact_local_cases_share_a_module_compiles_exactly()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = "import { css } from '@emotion/react'; const red = 'red'; export function topLevel() { const inner = { color: red }; return css(inner); } export function localValue() { const red = 'red'; const inner = { color: red }; return css(inner); } export function exactMath() { const gap = 2; const inner = { padding: gap * 4 }; return css(inner); }";
    // When
    let output = extract("probe-local.tsx", code, emotion_option())?;
    // Then
    let mut styles: Vec<(&str, &str)> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => (style.property.as_str(), style.value.as_str()),
            other => panic!("expected static style, got {other:?}"),
        })
        .collect();
    styles.sort_unstable();
    assert_eq!(styles, vec![("color", "red"), ("padding", "8px")]);
    Ok(())
}

#[rstest]
#[case("{ color: value, '&:hover': { padding: [2, value] } }", true)]
#[case("{ ...props }", false)]
#[case("{ [key]: value }", false)]
#[case("{ method() {} }", false)]
#[case("{ get color() { return 'red'; } }", false)]
#[case("{ __proto__: value }", false)]
#[case("{ padding: [...props] }", false)]
#[case("{ padding: [, value] }", true)]
fn runtime_shape_when_keys_are_fully_written_is_prepared(#[case] code: &str, #[case] known: bool) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&allocator);
    let source = format!("({code});");
    let parsed = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::tsx()).parse();
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expected expression");
    };
    // When
    let shape = super::local_rule_shapes::prepare(&builder, &statement.expression, &mut |_| None);
    // Then
    assert_eq!(shape.is_some(), known);
}
