use rstest::rstest;
use serial_test::serial;

use super::extracted;

#[rstest]
#[case("{ [require('@devup-ui/react/compat/stylex')]: value }", "require")]
#[case("{ [import('@devup-ui/react/compat/stylex')]: value }", "import(...)")]
#[case("{ getTheme = require('@devup-ui/react/compat/stylex') }", "require")]
#[case(
    "{ getTheme: { missing = require('@devup-ui/react/compat/stylex') } }",
    "require"
)]
#[case("[value = require('@devup-ui/react/compat/stylex')]", "require")]
#[case(
    "{ getTheme: [value = import('@devup-ui/react/compat/stylex')] }",
    "import(...)"
)]
#[case(
    "[...{ [require('@devup-ui/react/compat/stylex')]: value }]",
    "require"
)]
#[serial]
fn types_only_loader_errors_when_executed_in_an_accepted_binding_pattern(
    #[case] pattern: &str,
    #[case] loader: &str,
) {
    // Given
    let declaration = format!("const {pattern} = require('@devup-ui/react');");
    let code = format!("const before = 0;\n{declaration}");
    let column = declaration
        .find("require('@devup-ui/react/compat/stylex')")
        .or_else(|| declaration.find("import('@devup-ui/react/compat/stylex')"))
        .unwrap_or_else(|| panic!("fixture must contain the inner loader"))
        + 1;
    // When
    let error = match extracted(&code) {
        Err(error) => error,
        Ok(output) => panic!("binding-pattern loader must fail: {output}"),
    };
    // Then
    assert_eq!(
        error,
        format!(
            "test.tsx:2:{column}: `StyleX API()` cannot use `{loader}` at build time: compat/stylex has only type exports; use the real stylex entrypoint"
        )
    );
}

#[rstest]
#[case(
    "{ [pt.call(null, { top: 1 })]: value }",
    "pt.call",
    "invocation/binding member"
)]
#[case(
    "{ getTheme = pt.apply(null, [{ top: 1 }]) }",
    "pt.apply",
    "invocation/binding member"
)]
#[case("{ getTheme = pt.bind(null) }", "pt.bind", "invocation/binding member")]
#[case("{ getTheme = consume(pt) }", "pt", "argument passing")]
#[case("{ getTheme = { api: pt } }", "pt", "object storage")]
#[case("{ getTheme: [value = [pt]] }", "pt", "array storage")]
#[case("[...{ [consume(pt)]: value }]", "pt", "argument passing")]
#[serial]
fn known_api_escape_errors_when_executed_in_an_accepted_binding_pattern(
    #[case] pattern: &str,
    #[case] site: &str,
    #[case] form: &str,
) {
    // Given
    let declaration = format!("const {pattern} = require('@devup-ui/react');");
    let code = format!("import {{ positionTry as pt }} from '@stylexjs/stylex';\n{declaration}");
    let column = declaration
        .find(site)
        .unwrap_or_else(|| panic!("fixture must contain the API occurrence"))
        + 1;
    // When
    let error = match extracted(&code) {
        Err(error) => error,
        Ok(output) => panic!("binding-pattern API escape must fail: {output}"),
    };
    // Then
    assert_eq!(
        error,
        format!(
            "test.tsx:2:{column}: `stylex.positionTry()` cannot use `{site}` at build time: unsupported {form}; call the API directly"
        )
    );
}

#[rstest]
#[case("{ [key()]: value }", "[key()]: value")]
#[case("{ getTheme = fallback }", "getTheme = fallback")]
#[case("{ getTheme: { missing = fallback } }", "missing = fallback")]
#[case("[value = fallback]", "[value = fallback]")]
#[case("[...{ [key()]: value }]", "[key()]: value")]
#[case("{ getTheme, ...rest }", "...rest")]
#[serial]
fn ordinary_binding_patterns_remain_when_the_stylex_gate_is_active(
    #[case] pattern: &str,
    #[case] retained: &str,
) {
    // Given
    let code = format!(
        "import {{ positionTry as pt }} from '@stylexjs/stylex';\nconst {pattern} = require('@devup-ui/react'); const name = pt({{ top: '1px' }});"
    );
    // When
    let output = extracted(&code)
        .unwrap_or_else(|error| panic!("ordinary binding pattern must compile: {error}"));
    // Then
    assert!(output.contains(retained), "{output}");
    assert!(output.contains("require(\"@devup-ui/react\")"), "{output}");
    assert!(!output.contains("pt("), "{output}");
}

#[test]
#[serial]
fn mixed_root_pattern_keeps_its_runtime_default_when_stylex_is_consumed() {
    // Given
    let code = "const { stylex: sx, getTheme = fallback } = require('@devup-ui/react'); const name = sx.positionTry({ top: '1px' });";
    // When
    let output =
        extracted(code).unwrap_or_else(|error| panic!("mixed root pattern must compile: {error}"));
    // Then
    assert!(output.contains("getTheme = fallback"), "{output}");
    assert!(output.contains("require(\"@devup-ui/react\")"), "{output}");
    assert!(!output.contains("stylex: sx"), "{output}");
    assert!(!output.contains("positionTry("), "{output}");
}

#[rstest]
#[case(
    "const sx = require('@devup-ui/react/stylex'); const name = sx.positionTry({ top: '1px' });"
)]
#[case("const { positionTry: pt } = require('@stylexjs/stylex'); const name = pt({ top: '1px' });")]
#[case(
    "const root = require('@devup-ui/react'); const name = root.stylex.positionTry({ top: '1px' });"
)]
#[serial]
fn supported_outer_loader_binding_identifiers_are_not_api_reads(#[case] code: &str) {
    // Given: a supported direct loader and a consumed API call.
    // When
    let output = extracted(code)
        .unwrap_or_else(|error| panic!("supported outer loader must compile: {error}"));
    // Then
    assert!(output.contains("const name = \""), "{output}");
    assert!(!output.contains("positionTry("), "{output}");
    assert!(!output.contains("require("), "{output}");
}

#[rstest]
#[case(
    "{ getTheme = ((require) => require('@devup-ui/react/compat/stylex'))(loader) }",
    "require(\"@devup-ui/react/compat/stylex\")"
)]
#[case("{ [((pt) => pt.call(null))(fallback)]: value }", "pt.call(null)")]
#[case("{ getTheme = ((pt) => pt.bind(null))(fallback) }", "pt.bind(null)")]
#[serial]
fn shadowed_pattern_loaders_and_functions_remain_when_the_gate_is_active(
    #[case] pattern: &str,
    #[case] retained: &str,
) {
    // Given
    let code = format!(
        "import {{ positionTry as pt }} from '@stylexjs/stylex';\nconst {pattern} = require('@devup-ui/react'); const name = pt({{ top: '1px' }});"
    );
    // When
    let output = extracted(&code)
        .unwrap_or_else(|error| panic!("shadowed pattern expression must compile: {error}"));
    // Then
    assert!(output.contains(retained), "{output}");
    assert!(output.contains("require(\"@devup-ui/react\")"), "{output}");
    assert!(output.contains("const name = \""), "{output}");
}

#[test]
#[serial]
fn erased_pattern_types_are_skipped_when_the_gate_is_active() {
    // Given
    let code = "import { positionTry as pt } from '@stylexjs/stylex';
const { getTheme = fallback }: { getTheme: typeof pt; styles?: import('@devup-ui/react/compat/stylex').PositionTryStyles } = require('@devup-ui/react');
const name = pt({ top: '1px' });";
    // When
    let output = extracted(code)
        .unwrap_or_else(|error| panic!("erased binding types must compile: {error}"));
    // Then
    assert!(output.contains("getTheme = fallback"), "{output}");
    assert!(output.contains("require(\"@devup-ui/react\")"), "{output}");
    assert!(output.contains("const name = \""), "{output}");
}
