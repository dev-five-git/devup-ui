use crate::{ExtractOption, ResolvedModule, extract, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_snapshot;
use rstest::rstest;
use serial_test::serial;

/// The code and styles `source` compiles to, or the build error it reports
fn compile(source: &str) -> String {
    reset_class_map();
    reset_file_map();
    match extract("stylex.tsx", source, ExtractOption::default()) {
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
#[case::relations(
    "relations",
    "const s = stylex.create({ a: { color: { default: 'black', [stylex.when.ancestor(':hover')]: 'red', [stylex.when.descendant(':focus')]: 'blue', [stylex.when.anySibling(':active')]: 'green', [stylex.when.siblingBefore(':hover')]: 'pink', [stylex.when.siblingAfter('[data-open]')]: 'teal' } } });\nexport const p = stylex.props(stylex.defaultMarker(), s.a);"
)]
#[case::named_imports(
    "named_imports",
    "import { create, props, when, defaultMarker, defineMarker } from '@stylexjs/stylex';\nexport const card = defineMarker();\nconst s = create({ a: { [when.ancestor(':hover', card)]: { color: 'red' }, opacity: { default: 1, ':hover': { [when.descendant(':focus', defaultMarker())]: 0.5 } } } });\nexport const p = props(card, s.a);\nexport const q = props(defaultMarker());"
)]
#[case::custom_markers(
    "custom_markers",
    "export const card = stylex.defineMarker();\nconst row = stylex.defineMarker();\nexport { row };\nconst s = stylex.create({ a: { color: { default: 'black', [stylex.when.ancestor(':hover', card)]: 'red', [stylex.when.siblingAfter(':focus', row)]: 'blue', [stylex.when.ancestor(':active', 'my-class')]: 'green' } } });\nexport const p = stylex.props(card, row, s.a);"
)]
#[case::conditional_markers(
    "conditional_markers",
    "export const card = stylex.defineMarker();\nconst s = stylex.create({ a: { color: { default: 'black', [stylex.when.ancestor(':hover', card)]: 'red' } } });\nexport const f = (on) => stylex.props(on && card, on ? stylex.defaultMarker() : null, s.a);\nexport const g = stylex.props(card, 0, stylex.defaultMarker(), s.a);"
)]
#[case::dynamic_namespace(
    "dynamic_namespace",
    "const s = stylex.create({ a: (tone) => ({ color: { default: 'black', [stylex.when.ancestor(':hover')]: tone } }) });\nexport const f = (tone) => stylex.props(s.a(tone));"
)]
#[case::media_condition(
    "media_condition",
    "const s = stylex.create({ a: { backgroundColor: { default: null, '@media (min-width: 600px)': { [stylex.when.ancestor(':focus-visible')]: 'yellow' } } } });\nexport const p = stylex.props(s.a);"
)]
#[case::condition_errors(
    "condition_errors",
    "const s = stylex.create({ a: { color: { default: 'black', [stylex.when.ancestor('::before')]: 'red', [stylex.when.ancestor('hover')]: 'red', [stylex.when.ancestor('[open')]: 'red', [stylex.when.ancestor(pseudo)]: 'red', [stylex.when.ancestor(':hover', unknown)]: 'red', [stylex.when.ancestor()]: 'red', [stylex.when.near(':hover')]: 'red', [stylex.when.ancestor(':hover', marker.member)]: 'red' } } });"
)]
#[case::marker_errors(
    "marker_errors",
    "const local = stylex.defineMarker();\nexport const withArgument = stylex.defineMarker('x');\nexport default stylex.defineMarker();\nexport function f() { const inner = stylex.defineMarker(); return inner; }\nexport const d = stylex.defaultMarker('x');\nstylex.defineMarker();\nexport const { destructured } = stylex.defineMarker();"
)]
#[case::shadowed_export(
    "shadowed_export",
    "export const m = 1;\nexport function f() { const m = stylex.defineMarker(); return m; }"
)]
#[serial]
fn stylex_when(#[case] name: &str, #[case] code: &str) {
    let source = if code.starts_with("import {") {
        code.to_string()
    } else {
        format!("{IMPORT}{code}")
    };
    assert_snapshot!(name, compile(&source));
}

const MARKER_MODULE: &str = "/src/markers.stylex.ts";

fn resolve(specifier: &str, _importer: &str) -> Option<ResolvedModule> {
    (specifier == "./markers.stylex").then(|| ResolvedModule {
        path: MARKER_MODULE.to_string(),
        code: format!("{IMPORT}export const card = stylex.defineMarker();\nconst row = stylex.defineMarker();\nexport {{ row as other }};"),
    })
}

/// A marker another module defines names the class that module writes
#[test]
#[serial]
fn imported_markers_read_the_class_their_module_writes() {
    for single_css in [true, false] {
        reset_class_map();
        reset_file_map();
        let option = ExtractOption {
            single_css,
            ..ExtractOption::default()
        };
        let markers = extract_with_modules(
            MARKER_MODULE,
            &resolve("./markers.stylex", "").map_or_default(|module| module.code),
            option.clone(),
            false,
            &resolve,
        )
        .map_or_else(|error| error.to_string(), |output| output.code);
        let app = extract_with_modules(
            "/src/App.tsx",
            &format!(
                "{IMPORT}import {{ card, other }} from './markers.stylex';\nconst s = stylex.create({{ a: {{ color: {{ default: 'black', [stylex.when.ancestor(':hover', card)]: 'red', [stylex.when.descendant(':focus', other)]: 'blue' }} }} }});\nexport const A = () => <div {{...stylex.props(card, other, s.a)}} />;"
            ),
            option,
            false,
            &resolve,
        );
        let app = app.map_or_else(
            |error| error.to_string(),
            |output| format!("{}\n{:?}", output.code, output.styles),
        );
        for name in ["card", "row"] {
            let class = markers
                .split_once(&format!("const {name} = \""))
                .and_then(|(_, rest)| rest.split_once('"'))
                .map_or("", |(class, _)| class);
            assert!(!class.is_empty(), "{markers}");
            let classes = app
                .split_once("className=\"")
                .and_then(|(_, rest)| rest.split_once('"'))
                .map_or("", |(classes, _)| classes);
            assert!(
                classes.split(' ').any(|written| written == class),
                "{class}: {app}"
            );
            assert!(app.contains(&format!(".{class}:")), "{class}: {app}");
        }
        assert!(!app.contains("when"), "{app}");
    }
}
