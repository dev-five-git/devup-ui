use rstest::rstest;
use serial_test::serial;

use super::extracted;

#[rstest]
#[case("import { positionTry as api } from '@devup-ui/react/stylex';")]
#[case("import { positionTry as original } from '@stylexjs/stylex'; const api = original;")]
#[case("import * as sx from '@devup-ui/react/stylex'; const api = sx.positionTry;")]
#[case("import * as root from '@devup-ui/react'; const api = root.stylex.positionTry;")]
#[case("const { positionTry: api } = require('@devup-ui/react/stylex');")]
#[case("const root = require('@devup-ui/react'); const api = root.stylex.positionTry;")]
#[serial]
fn indirect_members_error_when_the_api_binding_is_known(#[case] binding: &str) {
    for member in ["call", "apply", "bind"] {
        for selection in [format!("api.{member}"), format!("api['{member}']")] {
            for use_site in [
                format!("{selection}(null, {{ top: 1 }});"),
                format!("const invoke = {selection};"),
            ] {
                // Given
                let code = format!("{binding}\n{use_site}");
                // When
                let error = match extracted(&code) {
                    Err(error) => error,
                    Ok(output) => panic!("indirect API must not survive: {output:?}"),
                };
                // Then
                assert!(error.contains("test.tsx:2:"), "{error}");
                assert!(error.contains("stylex.positionTry"), "{error}");
                assert!(error.contains(member), "{error}");
                assert!(error.contains("call the API directly"), "{error}");
            }
        }
    }
}

#[rstest]
#[case("consume(api);", "argument passing")]
#[case("const stored = { api };", "object storage")]
#[case("const stored = [api];", "array storage")]
#[case("const invoke = api[method];", "computed member")]
#[serial]
fn known_function_escape_names_api_and_form(#[case] use_site: &str, #[case] form: &str) {
    // Given
    let code = format!(
        "import {{ viewTransitionClass as api }} from '@devup-ui/react/stylex';\n{use_site}"
    );
    // When
    let error = match extracted(&code) {
        Err(error) => error,
        Ok(output) => panic!("known function escape must fail: {output:?}"),
    };
    // Then
    assert!(error.contains("test.tsx:2:"), "{error}");
    assert!(error.contains("stylex.viewTransitionClass"), "{error}");
    assert!(error.contains(form), "{error}");
    assert!(error.contains("call the API directly"), "{error}");
}

#[rstest]
#[case("consume(sx.viewTransitionClass);", "argument passing")]
#[case("const stored = { api: sx.viewTransitionClass };", "object storage")]
#[case("const stored = [sx.viewTransitionClass];", "array storage")]
#[serial]
fn namespace_function_escape_names_api_and_form(#[case] use_site: &str, #[case] form: &str) {
    // Given
    let code = format!("import * as sx from '@devup-ui/react/stylex';\n{use_site}");
    // When
    let error = match extracted(&code) {
        Err(error) => error,
        Ok(output) => panic!("namespace function escape must fail: {output:?}"),
    };
    // Then
    assert!(error.contains("test.tsx:2:"), "{error}");
    assert!(error.contains("stylex.viewTransitionClass"), "{error}");
    assert!(error.contains(form), "{error}");
    assert!(error.contains("call the API directly"), "{error}");
}

#[test]
#[serial]
fn binding_a_view_transition_function_errors_before_its_later_call() {
    // Given
    let code = "import { viewTransitionClass as vt } from '@devup-ui/react/stylex';
const bound = vt.bind(null);
export const result = bound({ old: { opacity: 0 } });";
    // When
    let error = match extracted(code) {
        Err(error) => error,
        Ok(output) => panic!("binding must not retain a runtime API: {output:?}"),
    };
    // Then
    assert!(error.contains("test.tsx:2:15:"), "{error}");
    assert!(error.contains("stylex.viewTransitionClass"), "{error}");
    assert!(error.contains("vt.bind"), "{error}");
    assert!(error.contains("call the API directly"), "{error}");
}

#[rstest]
#[case("require('@devup-ui/react/compat/stylex');")]
#[case("const sx = require('@devup-ui/react/compat/stylex');")]
#[case("const name = require('@devup-ui/react/compat/stylex').positionTry({});")]
#[case("let sx; sx = require('@devup-ui/react/compat/stylex');")]
#[case("consume(require('@devup-ui/react/compat/stylex'));")]
#[case("const x = [require('@devup-ui/react/compat/stylex')];")]
#[case("const x = { sx: require('@devup-ui/react/compat/stylex') };")]
#[case(
    "const { getTheme } = require('@devup-ui/react', require('@devup-ui/react/compat/stylex'));"
)]
#[case("const sx = import('@devup-ui/react/compat/stylex');")]
#[case("import('@devup-ui/react/compat/stylex');")]
#[serial]
fn types_only_runtime_loader_errors_at_its_occurrence(#[case] use_site: &str) {
    // Given
    let code = format!("const before = 0;\n{use_site}");
    let column = use_site
        .rfind("require(")
        .or_else(|| use_site.find("import("))
        .unwrap_or_else(|| panic!("fixture runtime loader"))
        + 1;
    // When
    let error = match extracted(&code) {
        Err(error) => error,
        Ok(output) => panic!("types-only runtime load must fail: {output:?}"),
    };
    // Then
    assert!(error.contains(&format!("test.tsx:2:{column}:")), "{error}");
    assert!(
        error.contains("compat/stylex has only type exports"),
        "{error}"
    );
}

#[test]
#[serial]
fn unrelated_function_members_and_shadowed_controls_remain_runtime_code() {
    // Given
    let code = "import { create, types } from '@devup-ui/react/stylex';
function user(create, require) { return [create.call(null), require('@devup-ui/react/compat/stylex')]; }
export const unrelated = [create.defineVars({}), types.defineVars({}), create['defineVars']({})];";
    // When
    let output = extracted(code)
        .unwrap_or_else(|error| panic!("ordinary members must remain supported: {error:?}"));
    // Then
    assert!(output.contains("create.call(null)"), "{output}");
    assert!(output.contains("create.defineVars({})"), "{output}");
    assert!(output.contains("types.defineVars({})"), "{output}");
    assert!(output.contains("create[\"defineVars\"]({})"), "{output}");
    assert!(
        output.contains("require(\"@devup-ui/react/compat/stylex\")"),
        "{output}"
    );
}

#[test]
#[serial]
fn erased_type_references_do_not_load_the_compatibility_source() {
    // Given
    let code = "import { positionTry as pt } from '@devup-ui/react/stylex'; const name = pt({});
import type { PositionTryStyles } from '@devup-ui/react/compat/stylex';
type Imported = import('@devup-ui/react/compat/stylex').PositionTryStyles;
export type { PositionTryStyles }; export const value = 1;";
    // When
    let output = extracted(code)
        .unwrap_or_else(|error| panic!("type references do not load runtime APIs: {error:?}"));
    // Then
    assert!(output.contains("export const value = 1"), "{output}");
}
