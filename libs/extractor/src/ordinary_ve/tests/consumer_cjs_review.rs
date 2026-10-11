use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;

#[rstest]
#[case("const step=2;exports.space=n=>n*step+'px';")]
#[case(concat!("const step=2;module.exports={", "space(n){return n*step+'px'},browser:window.document};"))]
#[serial]
fn selected_commonjs_callable_keeps_its_closure_when_native_input_calls_it(
    #[case] helper: &str,
) -> TestResult {
    // Given
    reset();
    let producer = "import {createTheme} from '@vanilla-extract/css';import {space} from './tokens';export const [theme,vars]=createTheme({space:space(4)});";
    let source = "import {css} from '@devup-ui/react';import {vars} from './producer';export const box=css({margin:vars.space});";
    // When
    let output = run(
        "/callable-cjs-consumer.ts",
        source,
        &[
            ("./producer", "/callable-producer.ts", producer),
            ("./tokens", "/callable-tokens.js", helper),
        ],
    )?;
    // Then
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn selected_commonjs_export_rejects_stale_data_when_excluded_code_receives_its_object() -> TestResult
{
    // Given
    reset();
    let helper = concat!(
        "module.exports={space:'8px'};\n",
        "external(module.exports);"
    );
    let producer = "import {createTheme} from '@vanilla-extract/css';import {space} from './tokens';export const [theme,vars]=createTheme({space});";
    let source = "import {css} from '@devup-ui/react';import {vars} from './producer';export const box=css({margin:vars.space});";
    let place = crate::locate(
        "/mutable-tokens.js",
        helper,
        helper
            .rfind("module.exports")
            .ok_or("handoff site missing")?,
    );
    // When
    let result = run(
        "/mutable-cjs-consumer.ts",
        source,
        &[
            ("./producer", "/mutable-cjs-producer.ts", producer),
            ("./tokens", "/mutable-tokens.js", helper),
        ],
    );
    // Then
    let error = match result {
        Ok(output) => panic!(
            "mutable CommonJS export published output: {} {:?}",
            output.code, output.styles
        ),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&place), "{error}");
    assert!(error.contains("may be changed"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn selected_commonjs_export_stays_exact_when_excluded_code_receives_only_its_primitive()
-> TestResult {
    // Given
    reset();
    let helper = concat!(
        "module.exports={space:'8px'};\n",
        "external(module.exports.space);"
    );
    let producer = "import {createTheme} from '@vanilla-extract/css';import {space} from './tokens';export const [theme,vars]=createTheme({space});";
    let source = "import {css} from '@devup-ui/react';import {vars} from './producer';export const box=css({margin:vars.space});";
    // When
    let output = run(
        "/primitive-cjs-consumer.ts",
        source,
        &[
            ("./producer", "/primitive-cjs-producer.ts", producer),
            ("./tokens", "/primitive-tokens.js", helper),
        ],
    )?;
    // Then
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    Ok(())
}
