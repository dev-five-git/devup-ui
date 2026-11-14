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

#[test]
#[serial_test::serial]
fn nested_nan_constant_stays_dynamic() {
    let output = extracted("import { Box } from '@devup-ui/react'; export function f() { const n = NaN; return <Box zIndex={n} />; }", "").unwrap_or_else(|error| panic!("nested_nan_constant_stays_dynamic: {error}"));
    assert!(output.code.contains("--"), "{}", output.code);
}

#[rstest::rstest]
#[case::unit("'a7'", Some("7"))]
#[case::surrogate_half("'\u{1F600}'", None)]
#[serial_test::serial]
fn nested_string_units_select_only_whole_characters(
    #[case] text: &str,
    #[case] expected: Option<&str>,
) {
    let index = usize::from(expected.is_some());
    let output = extracted(&format!("import {{ Box }} from '@devup-ui/react'; export function f() {{ const s = {text}; return <Box zIndex={{s[{index}]}} />; }}"), "").unwrap_or_else(|error| panic!("nested_string_units_select_only_whole_characters: {error}"));
    match expected {
        Some(expected) => assert_eq!(static_values(&output), vec![expected.to_string()]),
        None => assert!(output.code.contains("--"), "{}", output.code),
    }
}

#[test]
fn object_failure_is_the_one_under_the_first_key() {
    // Given an object holding several failed chains, in no source order
    let object = super::Constant::Object(std::rc::Rc::new(
        [
            ("b", super::Constant::Failed("second".into())),
            ("c", super::Constant::Number(1.0)),
            ("a", super::Constant::Failed("first".into())),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect(),
    ));
    // When its failure is retained, then the same one is every time
    assert!(
        matches!(object.failure(), Some(super::Constant::Failed(message)) if message == "first")
    );
}

#[rstest::rstest]
#[case::method_reading_this("const base = { p: 1, grow() { this.p = 2; } }; base.grow();", true)]
#[case::method_of_a_primitive("const base = 'a'; base.grow();", false)]
fn method_calls_change_only_what_can_change(#[case] source: &str, #[case] changed: bool) {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    let option = crate::ExtractOption::default();
    let changes = super::ChangeCheck::new(&parsed.program, "calls.tsx", &option, None);
    assert_eq!(changes.is_changed("base"), changed, "{source}");
}
