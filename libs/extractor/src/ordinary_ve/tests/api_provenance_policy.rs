use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run};
use super::mixed_support::{assert_consumed, has_static};

#[rstest]
#[case("let make=style;make=local;", "make=local", "make")]
#[case(concat!("const api={", "make:style};api.make=local;"), "api.make=local", "api.make")]
#[case(
    concat!("const api={", "make:style};const alias=api;alias.make=local;"),
    "alias.make=local",
    "api.make"
)]
#[case(
    concat!("const api={", "make:style};Object.assign(api,{", "make:local});"),
    "api,{make",
    "api.make"
)]
#[case("const api=ve;api.style=local;", "api.style=local", "api.style")]
#[serial]
fn unproven_api_writes_error_when_static_provenance_would_execute_a_stale_native_call(
    #[case] initialization: &str,
    #[case] site: &str,
    #[case] call: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from './api';import * as ve from '@vanilla-extract/css';\r\nconst 한글='😀';function local(){{return 'ordinary';}}\r\n{initialization}export const box={call}({{color:'stale-native'}});const browser=window.document;"
    );
    let place = crate::locate(
        "/api-write.ts",
        &source,
        source.find(site).ok_or("fixture mutation missing")?,
    );
    // When
    let result = run(
        "/api-write.ts",
        &source,
        &[(
            "./api",
            "/api.ts",
            "export {style} from '@vanilla-extract/css';",
        )],
    );
    // Then: the excluded write cannot silently retain the original native mapping.
    located_failure(result, &place, "may be changed");
    Ok(())
}

#[test]
#[serial]
fn computed_api_key_is_static_when_an_immutable_lexical_constant_proves_it() -> TestResult {
    // Given
    reset();
    let source = "import * as ve from './api';const key='style';export const box=ve[key]({color:'blue',padding:8});const browser=window.document;";
    // When
    let output = run(
        "/exact-key.ts",
        source,
        &[("./api", "/api.ts", "export * from '@vanilla-extract/css';")],
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!output.code.contains("ve[key]"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case("const key=window.name;", "ve[key]", "static API member")]
#[case(
    "let key='style';key=window.name;",
    "key=window.name",
    "may be changed"
)]
#[serial]
fn computed_api_key_errors_when_runtime_or_mutated_input_cannot_prove_the_terminal(
    #[case] binding: &str,
    #[case] site: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import * as ve from './api';\n{binding}export const box=ve[key]({{color:'stale-native'}});const browser=window.document;"
    );
    let place = crate::locate(
        "/unknown-key.ts",
        &source,
        source.find(site).ok_or("fixture key missing")?,
    );
    // When
    let result = run(
        "/unknown-key.ts",
        &source,
        &[("./api", "/api.ts", "export * from '@vanilla-extract/css';")],
    );
    // Then
    located_failure(result, &place, cause);
    Ok(())
}

#[test]
#[serial]
fn private_uncalled_native_helper_is_not_executed_when_no_surviving_use_escapes() -> TestResult {
    // Given
    reset();
    let source = "import {make} from './api';function privateHelper(){return make({color:window.name});}export const box=make({color:'blue'});const browser=window.document;";
    // When
    let output = run(
        "/private.ts",
        source,
        &[(
            "./api",
            "/api.ts",
            "export {style as make} from '@vanilla-extract/css';",
        )],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert_consumed(&output);
    assert!(!output.code.contains("privateHelper"), "{}", output.code);
    assert!(!output.code.contains("window.name"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case(
    "export function render(){return make({color:'red'});}",
    "render",
    "remains reachable at runtime"
)]
#[case("export const escaped=make;", "make;", "escapes")]
#[serial]
fn native_runtime_exports_error_when_a_terminal_barrel_does_not_make_them_build_time_data(
    #[case] escape: &str,
    #[case] site: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!("import {{make}} from './api';\r\n{escape}const browser=window.document;");
    let place = crate::locate(
        "/runtime-escape.ts",
        &source,
        source.find(site).ok_or("fixture escape missing")?,
    );
    // When
    let result = run(
        "/runtime-escape.ts",
        &source,
        &[(
            "./api",
            "/api.ts",
            "export {style as make} from '@vanilla-extract/css';",
        )],
    );
    // Then
    located_failure(result, &place, cause);
    Ok(())
}

#[test]
#[serial]
fn unexecuted_dynamic_branch_has_no_unknown_error_when_its_guard_is_known_false() -> TestResult {
    // Given
    reset();
    let source = "import * as ve from './api';function make(enabled){if(enabled)return ve[window.name]({color:window.document});return ve.style({color:'blue'});}export const box=make(false);const browser=window.document;";
    // When
    let output = run(
        "/branch.ts",
        source,
        &[("./api", "/api.ts", "export * from '@vanilla-extract/css';")],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert_eq!(output.styles.iter().filter(|value| matches!(value, crate::ExtractStyleValue::Static(style) if style.property == "color")).count(), 1);
    assert_consumed(&output);
    assert!(!output.code.contains("ve[window.name]"), "{}", output.code);
    Ok(())
}
