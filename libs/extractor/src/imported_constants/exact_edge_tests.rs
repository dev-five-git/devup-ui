use super::exact_tests::{extracted, static_values};

#[test]
#[serial_test::serial]
fn math_members_when_computed_names_are_literal() {
    for imported in [false, true] {
        let declaration = if imported {
            "import { N, C } from './values';"
        } else {
            "const N = Math['imul'](3, 4); const C = Math['PI'];"
        };
        let output = extracted(&format!("import {{ Box }} from '@devup-ui/react'; {declaration} export const view = <Box zIndex={{N}} opacity={{C}} />;"), "export const N = Math['imul'](3, 4); export const C = Math['PI'];").unwrap_or_else(|error| panic!("math_members_when_computed_names_are_literal: {error}"));
        let values = static_values(&output);
        assert!(values.contains(&"12".to_string()), "{values:?}");
        assert!(
            values.contains(&"3.141592653589793".to_string()),
            "{values:?}"
        );
    }
}

#[test]
#[serial_test::serial]
fn math_members_when_names_or_callable_values_are_unknown() {
    for expression in [
        "Math[key](1)",
        "Math.abs",
        "Math.PI(1)",
        "Math['random']()",
        "Math['UNKNOWN']",
    ] {
        assert!(extracted("import { css } from '@devup-ui/react'; import { N } from './values'; css({ zIndex: N });", &format!("export const N = {expression};")).is_err(), "{expression}");
    }
}

#[test]
#[serial_test::serial]
fn imported_values_when_nonfinite_numbers_stay_dynamic() {
    let output = extracted("import { Box } from '@devup-ui/react'; import { N } from './values'; export const view = <Box zIndex={N} />;", "export const N = Infinity;").unwrap_or_else(|error| panic!("imported_values_when_nonfinite_numbers_stay_dynamic: {error}"));
    assert!(output.code.contains("--"), "{}", output.code);
}

#[test]
#[serial_test::serial]
fn nested_constants_when_undefined_and_negative_numbers_select_styles() {
    let output = extracted("import { Box } from '@devup-ui/react'; export function f() { const empty = undefined; const n = -3; const alias = -n; const object = { value: 8 }; return <Box zIndex={empty ?? alias} color={empty ? 'red' : 'green'} />; }", "").unwrap_or_else(|error| panic!("nested_constants_when_undefined_and_negative_numbers_select_styles: {error}"));
    let values = static_values(&output);
    assert!(values.contains(&"3".to_string()), "{values:?}");
    assert!(values.contains(&"green".to_string()), "{values:?}");
}

#[test]
#[serial_test::serial]
fn enums_when_prior_siblings_include_exact_math() {
    let output = extracted("import { css } from '@devup-ui/react'; import { E } from './values'; css({ zIndex: E.C });", "enum Other { A = 4, B = A + 1 } export enum E { A = Math.imul(2, 3), B = Other.B + A, C = E.B + 1 }").unwrap_or_else(|error| panic!("enums_when_prior_siblings_include_exact_math: {error}"));
    assert_eq!(static_values(&output), vec!["12".to_string()]);
}
