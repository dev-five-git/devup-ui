use rustc_hash::FxHashMap;
use serial_test::serial;

use super::{visit, visit_with};

#[test]
#[serial]
fn stylex_follows_the_bindings_of_its_namespaces() {
    let visited = visit(
        "import * as stylex from '@stylexjs/stylex';\n\
         import { props } from '@stylexjs/stylex';\n\
         const styles = stylex.create({ base: { color: 'red' } });\n\
         export const a = stylex.props(styles.base);\n\
         export const b = props(styles.base);\n\
         export function c(stylex) { return stylex.props(styles.base); }\n\
         export function d(props) { return props(styles.base); }\n\
         export function e(styles) { return stylex.props(styles.base); }\n\
         export function f() { const styles = stylex.create({ other: { color: 'blue' } }); return stylex.props(styles.other); }",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 2);
    for expected in [
        "export const a = { className: \"a\" };",
        "export const b = { className: \"a\" };",
        "return stylex.props(styles.base);",
        "return props(styles.base);",
        "return { className: [styles.base].flat(Infinity).filter(Boolean).join(\" \") };",
        "return { className: \"b\" };",
    ] {
        assert!(
            visited.code.contains(expected),
            "{expected}: {}",
            visited.code
        );
    }
}

#[test]
#[serial]
fn stylex_variables_and_names_follow_the_bindings_that_hold_them() {
    let visited = visit(
        "import * as stylex from '@stylexjs/stylex';\n\
         const vars = stylex.defineVars({ color: 'red' });\n\
         const consts = stylex.defineConsts({ gap: '8px' });\n\
         const fade = stylex.keyframes({ from: { opacity: 0 } });\n\
         const theme = stylex.createTheme(vars, { color: 'blue' });\n\
         const styles = stylex.create({ base: { color: vars.color, marginTop: consts.gap, animationName: fade } });\n\
         const more = stylex.create({ extra: { ...stylex.include(styles.base), padding: 0 } });\n\
         export const a = stylex.props(theme, styles.base, more.extra);\n\
         export function f(vars) { return stylex.create({ other: { color: vars.color } }); }\n\
         export function g(fade) { return stylex.create({ other: { animationName: fade } }); }\n\
         export function h(styles) { return stylex.create({ other: { ...stylex.include(styles.base) } }); }\n\
         export function i(vars) { return stylex.createTheme(vars, { color: 'green' }); }",
    );
    assert_eq!(
        visited
            .errors
            .iter()
            .map(|error| error.split(" at build time").next().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec![
            "`stylex.create()` cannot use `vars.color`",
            "`stylex.create()` cannot use `fade`",
            "`stylex.include()` cannot use `styles.base`",
            "`stylex.createTheme()` cannot use `vars, { color: \"green\" }`",
        ]
    );
}

#[test]
#[serial]
fn stylex_values_imported_from_other_modules_bind_to_their_imports() {
    let visited = visit_with(
        "import * as stylex from '@stylexjs/stylex';\n\
         import { vars, darkTheme } from './vars.stylex';\n\
         const styles = stylex.create({ base: { color: vars.color } });\n\
         export const a = stylex.props(darkTheme, styles.base);\n\
         export function f(vars, darkTheme) { return stylex.props(darkTheme, stylex.create({ other: { color: vars.color } })); }",
        |visitor| {
            visitor.import_stylex(
                FxHashMap::from_iter([(
                    "vars".to_string(),
                    FxHashMap::from_iter([("color".to_string(), "--x".to_string())]),
                )]),
                FxHashMap::from_iter([
                    ("darkTheme".to_string(), "dark-theme".to_string()),
                    ("absent".to_string(), "none".to_string()),
                ]),
            );
        },
    );
    assert_eq!(visited.errors.len(), 1, "{:?}", visited.errors);
    assert!(
        visited.errors[0].contains("`stylex.create()` cannot use `vars.color`"),
        "{:?}",
        visited.errors
    );
    assert!(
        visited
            .code
            .contains("export const a = { className: \"dark-theme a\" };"),
        "{}",
        visited.code
    );
}
