use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ResolvedModule, extract, extract_with_modules};

#[rstest]
#[case("throw new Error('helper boom');", "helper boom")]
#[case("try { Date.now(); } catch {}", "Date")]
#[case("const effect = unknownRuntime();", "unknownRuntime")]
#[serial]
fn imported_module_effects_are_fatal_when_export_is_literal(
    #[case] effect: &str,
    #[case] message: &str,
) {
    // Given
    let helper = format!("export const color = 'red';\n{effect}");
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: helper.clone(),
        })
    };
    let source = "import { css } from '@devup-ui/react';\nimport { color } from './helper';\nexport const card = css({ color });";
    // When
    let Err(error) = extract_with_modules(
        "/src/card.css.ts",
        source,
        ExtractOption::default(),
        false,
        &resolver,
    ) else {
        panic!("helper effects must fail");
    };
    let error = error.to_string();
    // Then
    assert!(error.contains("/src/helper.ts:2:"), "{error}");
    assert!(error.contains(message), "{error}");
    assert!(!error.contains("not a callable"), "{error}");
}

#[rstest]
#[case("function unused() {}", "{ color: 'red' }", "color", "red")]
#[case(
    "function unused() { return Date.now(); }",
    "{ color: 'red' }",
    "color",
    "red"
)]
#[case("const unused = () => Date.now();", "{ color: 'red' }", "color", "red")]
#[case("const twice = (n) => n * 2;", "{ w: twice(3) }", "width", "24px")]
#[serial]
fn deterministic_css_stylesheets_match_regular_extraction(
    #[case] declaration: &str,
    #[case] rules: &str,
    #[case] property: &str,
    #[case] value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{ css }} from '@devup-ui/react'; {declaration} export const card = css({rules});"
    );
    // When
    let output = extract("/src/card.css.ts", &source, ExtractOption::default())?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style,
        crate::ExtractStyleValue::Static(style) if style.property == property && style.value == value
    )), "{:?}", output.styles);
    Ok(())
}

#[rstest]
#[case("css", "import { css } from '@devup-ui/react';")]
#[case("ui.css", "import * as ui from '@devup-ui/react';")]
#[serial]
fn pure_imported_literals_compile_when_module_is_nonthrowing(
    #[case] api: &str,
    #[case] import: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./helper").then(|| ResolvedModule {
            path: "/src/helper.ts".to_string(),
            code: "export const color = 'red'; function unused() { return Date.now(); }"
                .to_string(),
        })
    };
    let source = format!(
        "{import} import {{ color }} from './helper'; export const card = {api}({{ color }});"
    );
    // When
    let output = extract_with_modules(
        "/src/card.css.ts",
        &source,
        ExtractOption::default(),
        false,
        &resolver,
    )?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style,
        crate::ExtractStyleValue::Static(style) if style.property == "color" && style.value == "red"
    )), "{:?}", output.styles);
    assert_eq!(output.dependencies, vec!["/src/helper.ts"]);
    Ok(())
}

#[rstest]
#[case("function forbidden() { return Date.now(); }", "forbidden()")]
#[case(
    "const forbidden = () => { try { return Date.now(); } catch { return 1; } };",
    "forbidden()"
)]
#[serial]
fn called_forbidden_functions_fail_when_their_declarations_are_inert(
    #[case] declaration: &str,
    #[case] call: &str,
) {
    // Given
    let source = format!(
        "import {{ css }} from '@devup-ui/react'; {declaration} export const card = css({{ w: {call} }});"
    );
    // When
    let Err(error) = extract("/src/card.css.ts", &source, ExtractOption::default()) else {
        panic!("clock read must fail");
    };
    let error = error.to_string();
    // Then
    assert!(error.contains("/src/card.css.ts:1:"), "{error}");
    assert!(error.contains("Date"), "{error}");
}
