use super::*;
use serial_test::serial;

#[test]
#[serial]
fn literal_metadata_when_js_string_escape_precedes_value_locates_raw_token() {
    let source =
        "import {css} from '@devup-ui/react';\nconst a=css(\"content:'\\n';style-order:255\");";
    let message = error(source);
    let column = source
        .lines()
        .nth(1)
        .required("fixture has a second line")
        .find("255")
        .required("second line contains invalid order")
        + 1;
    assert!(
        message.starts_with(&format!("a.tsx:2:{column}:")),
        "{message}"
    );
}

#[test]
#[serial]
fn literal_metadata_when_imported_text_is_invalid_locates_original_reference() {
    let source =
        "import {css} from '@devup-ui/react';\nimport {rules} from './rules';\nconst a=css(rules);";
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/rules.ts".to_string(),
            code: "export const rules='style-order:255;color:red';".to_string(),
        })
    };
    let message = extract_with_modules(
        "/consumer.ts",
        source,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .required_err("imported invalid text must report a located error")
    .to_string();
    assert!(message.starts_with("/consumer.ts:3:13:"), "{message}");
}

#[test]
#[serial]
fn literal_metadata_when_static_key_is_interpolated_is_still_reserved() {
    let result =
        output("import {css} from '@devup-ui/react'; const a=css`${'style-order'}:2;color:red`; ");
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.style_order() == Some(2))));
}

#[test]
#[serial]
fn literal_metadata_when_order_is_quoted_finite_text_decodes_css_string() {
    let result = whole::evaluate(
        "import {css} from '@devup-ui/react'; const flag=()=>(trace.push('quoted'),false); const a=css`style-order:'${flag()?2:3}';color:red`;",
        "a",
    );
    assert_eq!(result.trace, serde_json::json!(["quoted"]));
    assert!(
        result
            .element
            .as_str()
            .required("quoted finite text must emit classes")
            .contains("--3-")
    );
}

#[test]
#[serial]
fn literal_metadata_when_css_variable_token_stream_contains_directive_preserves_data() {
    let result = output(
        "import {css} from '@devup-ui/react'; const a=css`--data:{style-order:2;};content:url('data:text/css;style-order:3');style-order:4;color:red`; ",
    );
    let names: Vec<_> = result
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.property()),
            _ => None,
        })
        .collect();
    assert!(names.contains(&"--data") && names.contains(&"content") && names.contains(&"color"));
    assert!(!names.contains(&"style-order"));
}
