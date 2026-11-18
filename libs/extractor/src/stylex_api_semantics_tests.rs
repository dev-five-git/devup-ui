use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_snapshot;
use rstest::rstest;
use serial_test::serial;

/// The code and CSS `source` compiles to, or the build error it reports
fn compile(filename: &str, source: &str) -> String {
    reset_class_map();
    reset_file_map();
    match extract(filename, source, ExtractOption::default()) {
        Ok(output) => {
            let mut styles: Vec<String> = output
                .styles
                .iter()
                .map(|style| format!("{style:?}"))
                .collect();
            styles.sort();
            format!("{}\n{}", output.code, styles.join("\n"))
        }
        Err(error) => error.to_string(),
    }
}

const IMPORT: &str = "import * as stylex from '@stylexjs/stylex';\n";

#[rstest]
#[case::contextual_parameters(
    "contextual_parameters",
    "const s = stylex.create({ w: (size, tone) => ({ color: { default: 'red', ':hover': size, '@media (min-width: 600px)': tone, ':focus': null }, width: size }) });\nexport const f = (w) => stylex.props(s.w(w, 'blue'));"
)]
#[case::contextual_attrs(
    "contextual_attrs",
    "const s = stylex.create({ w: (size) => ({ color: { default: size, ':hover': 'blue' } }) });\nexport const f = (w) => stylex.attrs(s.w(w));"
)]
#[case::contextual_runtime_leaf(
    "contextual_runtime_leaf",
    "const s = stylex.create({ w: (size) => ({ color: { default: size, ':hover': size + 1 } }) });\nexport const f = (w) => stylex.props(s.w(w));"
)]
#[case::constant_condition_key(
    "constant_condition_key",
    "const constants = stylex.defineConsts({ wide: '@media (min-width: 600px)', hover: ':hover' });\nconst s = stylex.create({ r: { color: { default: 'red', [constants.wide]: 'blue', [constants.hover]: 'green' } } });\nexport const p = stylex.props(s.r);"
)]
#[case::unknown_condition_key(
    "unknown_condition_key",
    "const s = stylex.create({ r: { color: { default: 'red', [later]: 'blue' } } });\nexport const p = stylex.props(s.r);"
)]
#[case::dashed_variable_names(
    "dashed_variable_names",
    "export const colors = stylex.defineVars({ '--global': 'black', local: 'red' });\nconst s = stylex.create({ r: { color: colors['--global'], backgroundColor: colors['local'] } });\nexport const p = stylex.props(s.r);"
)]
#[case::nested_member_value(
    "nested_member_value",
    "const s = stylex.create({ r: { color: deep.nested.value } });\nexport const p = stylex.props(s.r);"
)]
#[case::typed_variables(
    "typed_variables",
    "export const v = stylex.defineVars({ count: stylex.types.integer(3), tone: stylex.types.color({ default: 'red', '@media (prefers-color-scheme: dark)': 'white' }), plain: 'x' });"
)]
#[case::typed_variable_errors(
    "typed_variable_errors",
    "export const v = stylex.defineVars({ half: stylex.types.integer(3.5), printed: stylex.types.color({ '@media print': 'red' }), odd: stylex.types.wat('x') });"
)]
#[case::position_try_names(
    "position_try_names",
    "const fallback = stylex.positionTry({ positionAnchor: '--anchor', top: '0' });\nconst other = stylex.positionTry({ bottom: '0' });\nconst s = stylex.create({ r: { positionTryFallbacks: fallback, anchorName: '--anchor' }, o: { positionTryFallbacks: other } });\nexport const p = stylex.props(s.r, s.o);"
)]
#[case::view_transition_groups(
    "view_transition_groups",
    "const fade = stylex.viewTransitionClass({ group: { transitionTimingFunction: 'ease' }, imagePair: { isolation: 'auto' }, old: { animationDuration: '2s' }, new: { animationDuration: '1s' } });\nconst s = stylex.create({ r: { viewTransitionClass: fade } });\nexport const p = stylex.props(s.r);"
)]
#[case::view_transition_errors(
    "view_transition_errors",
    "export const fade = stylex.viewTransitionClass({ animationDuration: '2s', old: 'x', new: { opacity: later }, ...rest, [k]: {} });"
)]
#[serial]
fn stylex_api_semantics(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile("stylex.tsx", &format!("{IMPORT}{code}")));
}
