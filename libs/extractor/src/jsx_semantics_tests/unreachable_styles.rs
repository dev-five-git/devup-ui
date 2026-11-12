use super::*;
use rstest::rstest;
use serial_test::serial;

const APIS: &str = "import { Box, css, styled } from '@devup-ui/react';\n";

// A value the build knows emits only the styles it can select
#[rstest]
#[case("const a = (c) => <Box opacity={[1, 0.5]['10']} />;", &[])]
#[case("const a = (c) => <Box opacity={[1, 0.5, 0.2][c ? 0 : 1]} />;", &["1", ".5"])]
#[case("const a = (c) => <Box opacity={[1, 0.5, 0.2][c ? 5 : 1]} />;", &[".5"])]
#[case("const a = (c) => <Box opacity={({ a: 1, a: 0.5 }).a} />;", &[".5"])]
#[case("const a = (c) => <Box opacity={!0 ? 1 : 0.5} />;", &["1"])]
#[case("const a = (c) => <Box opacity={(0, 0.5)} />;", &[".5"])]
#[case("const a = (c) => <Box styleOrder={2} opacity={[1, 0.5][0 + 1]} />;", &[".5"])]
#[case("const a = (c) => jsx(Box, { opacity: [1, 0.5]['10'] });", &[])]
#[case("const a = (c) => jsx(Box, { styleOrder: 2, opacity: !0 ? 1 : 0.5 });", &["1"])]
#[case("const a = (c) => <Box css={{ opacity: [1, 0.5]['1'] }} />;", &[".5"])]
#[case("const a = (c) => jsx(Box, { css: { opacity: [1, 0.5]['1'] } });", &[".5"])]
#[case("const a = css({ opacity: [1, 0.5][0 + 1] });", &[".5"])]
#[case("const a = css`opacity: ${!0 ? 1 : 0.5};`;", &["1"])]
#[case("const A = styled.div({ opacity: !0 ? 1 : 0.5 });", &["1"])]
#[case("const A = styled.div({ opacity: [1, 0.5][+5] });", &[])]
#[case("const A = styled.div({ opacity: void 0 });", &[])]
#[serial]
fn known_values_emit_only_the_styles_they_select(#[case] code: &str, #[case] expected: &[&str]) {
    let source = format!("{APIS}{JSX_RUNTIME}export {code}");
    let compiled = output(&source);
    let mut expected: Vec<_> = expected
        .iter()
        .map(|value| ("opacity".to_string(), (*value).to_string()))
        .collect();
    expected.sort();
    assert_eq!(static_styles(&compiled), expected, "{}", compiled.code);
    // No value is left to the runtime either
    assert!(!compiled.code.contains("\"--"), "{}", compiled.code);
}

// `undefined`, which a binding may name, is read where it is evaluated
#[rstest]
#[case("undefined ?? c", true)]
#[case("undefined || c", true)]
#[case("undefined && c", false)]
#[case("undefined ? c : 0.5", true)]
#[serial]
fn undefined_tests_select_what_they_read(#[case] value: &str, #[case] applied: bool) {
    let source = format!("{BOX}export const render = (c) => <Box opacity={{{value}}} />;");
    let rendered = whole::evaluate(&source, "render(0.5)");
    let class = rendered.element["props"]["className"]
        .as_str()
        .unwrap_or("");
    assert_eq!(!class.is_empty(), applied, "{}", output(&source).code);
}

// ... and around values read by calls, which it is captured with
#[rstest]
#[case("undefined ?? read(0.5)", true)]
#[case("undefined || read(0.5)", true)]
#[case("undefined && read(0.5)", false)]
#[serial]
fn captured_undefined_tests_select_what_they_read(#[case] value: &str, #[case] applied: bool) {
    let source = format!(
        "{BOX}export const render = (read) => <Box id={{read('id')}} opacity={{{value}}} title={{read('title')}} />;"
    );
    let rendered = whole::evaluate(&source, "render((value) => (trace.push(value), value))");
    let class = rendered.element["props"]["className"]
        .as_str()
        .unwrap_or("");
    assert_eq!(!class.is_empty(), applied, "{}", output(&source).code);
    assert_eq!(rendered.trace[0], "id");
    assert_eq!(
        rendered.trace[rendered.trace.as_array().map_or(0, Vec::len) - 1],
        "title"
    );
}

// A key no literal fold reads is read by the runtime
#[test]
#[serial]
fn styled_lookups_the_build_cannot_fold_run_as_written() {
    let compiled = output(&format!(
        "{APIS}export const A = styled.div({{ opacity: [1, 0.5]['length'] }});"
    ));
    assert!(static_styles(&compiled).is_empty(), "{}", compiled.code);
    assert!(compiled.code.contains("\"length\""), "{}", compiled.code);
}

// A conditional key reads what the table holds there, as the runtime does
#[rstest]
#[case("<Box opacity={[1, 0.5, 0.2][c ? 0 : 1]} />")]
#[case("<Box opacity={[1, 0.5, 0.2][c ? 5 : 1]} />")]
#[case("<Box opacity={({ a: 1, b: 0.5, d: 0.2 })[c ? 'a' : 'b']} />")]
#[serial]
fn conditional_keys_select_at_runtime_what_they_read(#[case] element: &str) {
    let source = format!("{BOX}export const render = (c) => {element};");
    let class = |probe: &str| {
        whole::evaluate(&source, probe).element["props"]["className"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };
    let (on, off) = (class("render(true)"), class("render(false)"));
    assert_ne!(on, off);
}

// Props a conditional spread names next to its styles reach the element in
// the order written, around values read by calls
#[test]
#[serial]
fn called_conditional_spreads_read_their_props_in_order() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const render = (c, read) => jsx(Box, {{ id: read('id'), ...(c ? {{ p: 1, title: read('title') }} : null), m: read('m') }});"
    );
    let read = "(value) => (trace.push(value), value)";
    let on = whole::evaluate(&source, &format!("render(true, {read})"));
    assert_eq!(on.trace, serde_json::json!(["id", "title", "m"]));
    assert_eq!(on.element["props"]["id"], "id");
    assert_eq!(on.element["props"]["title"], "title");
    assert!(on.element["props"]["p"].is_null());
    let off = whole::evaluate(&source, &format!("render(false, {read})"));
    assert_eq!(off.trace, serde_json::json!(["id", "m"]));
    assert!(off.element["props"]["title"].is_null());
    assert!(output(&source).code.contains("__devupBranch"));
}
