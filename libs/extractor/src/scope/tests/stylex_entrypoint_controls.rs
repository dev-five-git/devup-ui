use serial_test::serial;

use super::{extracted, visit};

#[test]
#[serial]
fn legacy_root_default_keeps_generic_box_and_style_routes() {
    let source = "import D from '@devup-ui/react';
export const box = <D.Box bg='red' />;
export const Styled = D.styled.div({ color: 'blue' });
export const global = <D.Global styles={{ body: { m: 0 } }} />;
export const runtime = D.getTheme();";
    let output = visit(source);
    assert_eq!(output.errors, Vec::<String>::new());
    assert!(output.code.contains("<div className="), "{}", output.code);
    assert!(!output.code.contains("D.styled"), "{}", output.code);
    assert!(output.code.contains("<D.Global />"), "{}", output.code);
    assert!(output.code.contains("D.getTheme()"), "{}", output.code);
}

#[test]
#[serial]
fn runtime_only_root_destructuring_keeps_default_and_nested_patterns() {
    let cases = [
        "const { getTheme = () => 'fallback' } = require('@devup-ui/react'); export const actual = typeof getTheme;",
        "const { a: { b } } = require('@devup-ui/react'); export const actual = typeof b;",
        "const [x] = require('@devup-ui/react'); export const actual = typeof x;",
    ];
    for source in cases {
        let output = extracted(source).unwrap_or_else(|error| panic!("{error}"));
        assert!(output.contains("require("), "{output}");
        assert!(output.contains("typeof"), "{output}");
    }
}

#[test]
#[serial]
fn mixed_root_destructuring_preserves_runtime_default_when_stylex_is_consumed() {
    let source = "const { stylex: sx, getTheme = () => 'fallback' } = require('@devup-ui/react'); export const actual = typeof getTheme; export const name = sx.positionTry({ top: '1px' });";
    let output = extracted(source).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains("getTheme ="), "{output}");
    assert!(output.contains("require("), "{output}");
    assert!(!output.contains("positionTry("), "{output}");
    assert!(!output.contains("stylex: sx"), "{output}");
}

#[test]
#[serial]
fn immediate_and_assigned_root_loaders_cannot_leave_known_stylex_calls() {
    let cases = [
        "export const name = require('@devup-ui/react').stylex.positionTry({ top: '1px' });",
        "let root; root = require('@devup-ui/react'); export const name = root.stylex.positionTry({ top: '1px' });",
        "let sx; sx = require('@devup-ui/react').stylex; export const name = sx.positionTry({ top: '1px' });",
        "export const name = require('@devup-ui/react')['stylex'].positionTry({ top: '1px' });",
    ];
    for source in cases {
        let error = extracted(source)
            .err()
            .unwrap_or_else(|| panic!("unsupported loader projection must fail"));
        assert!(error.contains("test.tsx:"), "{error}");
        assert!(error.contains("cannot use"), "{error}");
    }
}

#[test]
#[serial]
fn direct_runtime_root_member_and_ignored_loader_are_not_stylex_escapes() {
    let source =
        "require('@devup-ui/react'); export const theme = require('@devup-ui/react').getTheme();";
    let output = extracted(source).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains(".getTheme()"), "{output}");
    assert!(output.contains("require("), "{output}");
}

#[test]
#[serial]
fn shadowed_root_loader_stays_user_code_in_immediate_and_assignment_forms() {
    let source = "function user(require) { let root; root = require('@devup-ui/react'); return [require('@devup-ui/react').stylex.positionTry({ top: '1px' }), root.stylex.positionTry({ top: '1px' })]; }";
    let output = extracted(source).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains(".stylex.positionTry("), "{output}");
    assert!(output.contains("root = require("), "{output}");
}

#[test]
#[serial]
fn legacy_root_default_does_not_advertise_a_stylex_default_entrypoint() {
    let source = "import D from '@devup-ui/react'; export const name = D.stylex.positionTry({ top: '1px' });";
    let error = extracted(source)
        .err()
        .unwrap_or_else(|| panic!("legacy generic default is not a StyleX source"));
    assert!(error.contains("test.tsx:"), "{error}");
    assert!(error.contains("D.stylex"), "{error}");
}

#[test]
#[serial]
fn generic_root_namespace_reexports_keep_the_existing_barrel_route() {
    let source = "export * as UI from '@devup-ui/react';";
    let output = extracted(source).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains("export * as UI"), "{output}");
}

#[test]
#[serial]
fn dedicated_stylex_namespace_reexports_report_a_located_escape() {
    let error = extracted("export * as sx from '@devup-ui/react/stylex';")
        .err()
        .unwrap_or_else(|| panic!("a dedicated API namespace cannot escape"));
    assert!(error.contains("test.tsx:1:"), "{error}");
    assert!(error.contains("star re-exports"), "{error}");
}
