use crate::{ExtractOption, extract_without_source_map, vanilla_extract::execute_stylesheet};
use serial_test::serial;

#[rstest::rstest]
#[case("style({ get width() { throw new Error('style getter boom'); } })")]
#[case("keyframes({ from: { get opacity() { throw new Error('style getter boom'); } } })")]
#[case("fontFace({ get src() { throw new Error('style getter boom'); } })")]
#[serial]
fn serialization_errors_are_not_replaced_with_empty_styles(#[case] call: &str) {
    let source = format!(
        "import {{ style, keyframes, fontFace }} from '@devup-ui/react';\nexport const card = {call};"
    );

    let error = execute_stylesheet(
        &source,
        "/src/getter.css.ts",
        &ExtractOption::default(),
        None,
    )
    .err()
    .unwrap_or_default();

    assert!(error.starts_with("/src/getter.css.ts:2:"), "{error}");
    assert!(error.contains("style getter boom"), "{error}");
}

#[test]
#[serial]
fn style_names_cannot_overwrite_pending_placeholder_entries() -> Result<(), String> {
    let source = "import { style } from '@devup-ui/react';\nexport const __style_1__ = style({ color: 'red' });\nexport const second = style({ color: 'blue' });";

    let (collected, _) = execute_stylesheet(
        source,
        "/src/placeholders.css.ts",
        &ExtractOption::default(),
        None,
    )?;

    assert_eq!(collected.styles.len(), 2);
    assert_eq!(collected.styles["__style_1__"].json, "{\"color\":\"red\"}");
    assert_eq!(collected.styles["second"].json, "{\"color\":\"blue\"}");
    Ok(())
}

#[rstest::rstest]
#[case("null.color")]
#[case("[...null]")]
#[case("({}).missing.color")]
#[serial]
fn static_dispatch_cannot_ignore_throwing_non_style_initializers(#[case] expression: &str) {
    let source = format!(
        "import {{ css }} from '@devup-ui/react';\nconst unused = {expression};\nexport const red = css({{ color: 'red' }});"
    );

    let error = extract_without_source_map(
        "/src/throwing-data.css.ts",
        &source,
        ExtractOption::default(),
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();

    assert!(error.starts_with("/src/throwing-data.css.ts:2:"), "{error}");
    assert!(error.contains("TypeError"), "{error}");
}

#[rstest::rstest]
#[case("Boolean(Intl)", "Intl")]
#[case("typeof Intl === 'object'", "Intl")]
#[case("performance.memory", "performance")]
#[serial]
fn environment_reads_cannot_be_replaced_with_synthetic_build_values(
    #[case] expression: &str,
    #[case] global: &str,
) {
    let source = format!(
        "import {{ style }} from '@devup-ui/react';\nexport const card = style({{ width: ({expression}) ? '10px' : '20px' }});"
    );

    let error = extract_without_source_map(
        "/src/environment-read.css.ts",
        &source,
        ExtractOption::default(),
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();

    assert!(
        error.starts_with("/src/environment-read.css.ts:2:"),
        "{error}"
    );
    assert!(error.contains(global), "{error}");
    assert!(error.contains("Fix:"), "{error}");
}

#[test]
#[serial]
fn function_source_reflection_cannot_observe_build_instrumentation() {
    let source = "import { style } from '@devup-ui/react';\nfunction value() { return Math.PI; }\nexport const card = style({ color: value.toString().includes('__devup_read_site_') ? 'blue' : 'red' });";

    let error = extract_without_source_map(
        "/src/function-source.css.ts",
        source,
        ExtractOption::default(),
    )
    .err()
    .map(|error| error.to_string())
    .unwrap_or_default();

    assert!(
        error.starts_with("/src/function-source.css.ts:3:"),
        "{error}"
    );
    assert!(error.contains("Function.prototype.toString"), "{error}");
    assert!(error.contains("transforms the function"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
}
