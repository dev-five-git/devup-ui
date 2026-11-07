use super::*;
use crate::extract_style::{
    extract_static_style::ExtractStaticStyle, style_property::StyleProperty,
};

#[rstest]
#[case(
    "const inner = [{ padding: props.padding, lineHeight: props.lineHeight }]; return <div css={inner} />;",
    "inner[\"0\"][\"padding\"]",
    "inner[\"0\"][\"lineHeight\"]"
)]
#[case(
    "const inner = [...[...[{ padding: props.padding, lineHeight: props.lineHeight }]]]; return <div css={inner} />;",
    "inner[\"0\"][\"padding\"]",
    "inner[\"0\"][\"lineHeight\"]"
)]
#[case(
    "return <div css={[...[{ padding: props.padding, lineHeight: props.lineHeight }]]} />;",
    "props.padding",
    "props.lineHeight"
)]
#[case(
    "return <div css={[, ...[, ...[{ padding: props.padding, lineHeight: props.lineHeight }]]]} />;",
    "props.padding",
    "props.lineHeight"
)]
#[serial]
fn css_variables_when_exact_arrays_compose_keep_numeric_roles(
    #[case] body: &str,
    #[case] padding: &str,
    #[case] line_height: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code =
        format!("import {{ css }} from '@emotion/react'; export function f(props) {{ {body} }}");
    // When
    let output = extract("local-array.tsx", &code, emotion_option())?;
    // Then
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
            .count(),
        2
    );
    crate::style_slot_test_support::assert_variables(
        &output,
        "local-array.tsx",
        &emotion_option(),
        &[
            ("padding", crate::style_slot_test_support::numeric(padding)),
            ("line-height", line_height.to_owned()),
        ],
    );
    assert_eq!(
        output.code.matches("typeof __devupValue").count(),
        1,
        "{}",
        output.code
    );
    Ok(())
}

#[rstest]
#[case("return <div css={[...props.rules]} />;")]
#[case("return <div css={[...[...props.rules]]} />;")]
#[case("const inner = [...props.rules]; return <div css={inner} />;")]
#[case(
    "const inner = [{ color: props.color }]; inner[0].color = 'red'; return <div css={inner} />;"
)]
#[case("const inner = [{ color: props.color }]; consume(inner); return <div css={inner} />;")]
#[case("const inner = [{ color: tone }]; const tone = 'red'; return <div css={inner} />;")]
#[serial]
fn located_error_when_array_shape_is_unknown_changed_escaped_or_tdz(#[case] body: &str) {
    // Given
    reset_class_map();
    reset_file_map();
    let code =
        format!("import {{ css }} from '@emotion/react';\nexport function f(props) {{ {body} }}");
    // When
    let Err(error) = extract("local-array.tsx", &code, emotion_option()) else {
        panic!("expected located error");
    };
    let error = error.to_string();
    // Then
    assert!(error.starts_with("local-array.tsx:2:"), "{error}");
}

#[rstest]
#[case("(p) => p.tone", None)]
#[case("function(p) { return p.tone; }", None)]
#[case("(p) => p.theme.colors.brand", Some("var(--colors-brand)"))]
#[serial]
fn styled_callback_when_local_matches_inline_semantics(
    #[case] callback: &str,
    #[case] theme: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = |local: bool| {
        let rules = format!("{{ color: {callback} }}");
        let declaration = if local {
            format!("const inner = {rules}; const Inner = styled.div(inner);")
        } else {
            format!("const Inner = styled.div({rules});")
        };
        format!(
            "import {{ styled }} from '@devup-ui/react'; export function f() {{ {declaration} return <Inner tone='purple' />; }}"
        )
    };
    reset_class_map();
    reset_file_map();
    let direct = extract("local-function.tsx", &source(false), emotion_option())?;
    reset_class_map();
    reset_file_map();
    // When
    let local = extract("local-function.tsx", &source(true), emotion_option())?;
    // Then
    assert_eq!(local.styles, direct.styles);
    assert!(!local.code.contains(":inner[`color`]"), "{}", local.code);
    if let Some(theme) = theme {
        assert!(
            local.styles.iter().any(
                |style| matches!(style, ExtractStyleValue::Static(style) if style.value == theme)
            ),
            "{local:?}"
        );
    } else {
        assert!(local.code.contains("tone=\"purple\""), "{}", local.code);
        assert!(
            local.code.contains("\"tone\": __devupRecord"),
            "{}",
            local.code
        );
        assert!(local.code.contains(".tone"), "{}", local.code);
    }
    Ok(())
}

#[test]
#[serial]
fn callbacks_when_passed_as_ordinary_props_keep_original_identity()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = "import { Box } from '@devup-ui/react'; export function f() { const inner = { color: 'red', onClick: () => click(), 'data-format': () => format() }; return <Box as={Custom} {...inner} />; }";
    // When
    let output = extract("local-callback.tsx", code, emotion_option())?;
    // Then
    assert_eq!(
        output.styles,
        std::iter::once(ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None
        )))
        .collect()
    );
    let bucket = crate::css_bucket("local-callback.tsx", &emotion_option());
    let Some(style) = output.styles.iter().next() else {
        panic!("one static style");
    };
    let Some(metadata) = style.extract(bucket.as_deref()) else {
        panic!("class metadata");
    };
    let StyleProperty::ClassName(class_name) = metadata else {
        panic!("expected static callback class");
    };
    assert!(output.code.contains("return DevupAs0 = Custom || \"div\", __devupValue0 = inner[\"onClick\"], __devupValue1 = inner[\"data-format\"], <DevupAs0"), "{}", output.code);
    assert!(
        output.code.contains(&format!("className=\"{class_name}\"")),
        "{}",
        output.code
    );
    assert_eq!(output.code.matches("click()").count(), 1, "{}", output.code);
    assert_eq!(
        output.code.matches("format()").count(),
        1,
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn array_leaves_when_captured_use_original_paths_once() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = "import { css } from '@emotion/react'; export function f(props) { const inner = [...[{ padding: props.getPadding(), lineHeight: props.lineHeight }]]; return <div css={inner} />; }";
    // When
    let output = extract("local-array.tsx", code, emotion_option())?;
    // Then
    assert_eq!(
        output.code.matches("props.getPadding()").count(),
        1,
        "{}",
        output.code
    );
    assert_eq!(
        output.code.matches("props.lineHeight").count(),
        1,
        "{}",
        output.code
    );
    crate::style_slot_test_support::assert_variables(
        &output,
        "local-array.tsx",
        &emotion_option(),
        &[
            (
                "padding",
                crate::style_slot_test_support::numeric("inner[\"0\"][\"padding\"]"),
            ),
            ("line-height", "inner[\"0\"][\"lineHeight\"]".to_owned()),
        ],
    );
    Ok(())
}
