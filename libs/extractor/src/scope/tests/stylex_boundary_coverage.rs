use rstest::rstest;
use serial_test::serial;

use super::extracted;

#[rstest]
#[case(
    "const { positionTry: pt, ...rest } = require('@devup-ui/react/stylex');",
    "rest"
)]
#[case(
    "const { default: sx } = require('@devup-ui/react/stylex');",
    "advertised named"
)]
#[case(
    "const { positionTry: { nested } } = require('@devup-ui/react/stylex');",
    "nested patterns"
)]
#[case(
    "const { ['positionTry']: pt } = require('@devup-ui/react/stylex');",
    "computed"
)]
#[case(
    "const [sx] = require('@devup-ui/react/stylex');",
    "namespace identifier"
)]
#[case(
    "const sx = require('@devup-ui/react/compat/stylex');",
    "only type exports"
)]
#[case(
    "import { positionTry } from '@devup-ui/react/compat/stylex';",
    "only type exports"
)]
#[case(
    "import * as root from '@devup-ui/react'; const x = <root.stylex />;",
    "rendered"
)]
#[case("export { positionTry } from '@devup-ui/react/stylex';", "re-exports")]
#[case("export { stylex } from '@devup-ui/react';", "re-exports")]
#[serial]
fn unsupported_source_patterns_report_their_contract(
    #[case] code: &str,
    #[case] requirement: &str,
) {
    // When
    let error = match extracted(code) {
        Err(error) => error,
        Ok(output) => panic!("unsupported source pattern must fail: {output:?}"),
    };
    // Then
    assert!(error.contains("test.tsx:1:"), "{error}");
    assert!(error.contains(requirement), "{error}");
}

#[rstest]
#[case(
    "import * as root from '@devup-ui/react'; export type { root }; export const x = root.stylex.positionTry({});"
)]
#[case("export type { PositionTryStyles } from '@devup-ui/react/stylex';")]
#[case("export { type PositionTryStyles } from '@devup-ui/react/stylex';")]
#[serial]
fn type_exports_remain_erased_when_residual_boundary_runs(#[case] code: &str) {
    // Given a value source to force extraction even for type-only re-exports.
    let code = format!(
        "import {{ positionTry as pt }} from '@devup-ui/react/stylex'; const compiled = pt({{}}); {code}"
    );
    // When
    let output = extracted(&code)
        .unwrap_or_else(|error| panic!("type-only exports are not runtime escapes: {error:?}"));
    // Then
    assert!(output.contains("type"), "{output}");
    assert!(!output.contains("pt("), "{output}");
}

#[test]
#[serial]
fn root_loader_arity_is_checked_before_namespace_registration() {
    // Given
    let code = "const root = require('@devup-ui/react', options); root.stylex.positionTry({});";
    // When
    let error = match extracted(code) {
        Err(error) => error,
        Ok(output) => panic!("root loader has unsupported arity: {output:?}"),
    };
    // Then
    assert!(
        error.contains("one literal module-source argument"),
        "{error}"
    );
}
