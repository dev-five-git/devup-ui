use std::collections::HashMap;

use css::file_map::reset_file_map;
use serial_test::serial;

use crate::import_alias_visit::{Edit, transform_import_aliases_with_edits};
use crate::vanilla_extract::{Stylesheet, execute_located};
use crate::{ExtractOption, ImportAlias, ResolvedModule};

mod css_paths;
mod css_reads;
mod regressions;
mod retained_css;
mod sandbox;
mod side_effect_dependency;

type Files = &'static [(&'static str, &'static str)];

const PACKAGE: &str = "@devup-ui/react";

fn resolve(files: Files, specifier: &str) -> Option<ResolvedModule> {
    files
        .iter()
        .find(|(path, _)| {
            path.trim_end_matches(".ts")
                == specifier.trim_start_matches('.').trim_end_matches(".ts")
        })
        .map(|(path, code)| ResolvedModule {
            path: (*path).to_string(),
            code: (*code).to_string(),
        })
}

/// The error evaluating `code` gives, `files` resolving what it imports
fn error_of(filename: &str, code: &str, files: Files) -> String {
    reset_file_map();
    let resolver = move |specifier: &str, _: &str| resolve(files, specifier);
    execute_located(
        Stylesheet {
            filename,
            code,
            source: code,
            edits: &[],
        },
        &ExtractOption::default(),
        Some(&resolver),
    )
    .err()
    .unwrap_or_default()
}

/// `error` split into what leads it, before ` Fix: `, the fix and its call
/// stack
fn parts(error: &str) -> (&str, &str, &str) {
    let (lead, rest) = error.split_once(". Fix: ").unwrap_or((error, ""));
    let (fix, calls) = rest.split_once('\n').unwrap_or((rest, ""));
    (lead, fix, calls)
}

#[test]
#[serial]
fn an_error_points_at_the_code_as_written() {
    let error = error_of(
        "/a.css.ts",
        "import { createVar } from '@devup-ui/react';\nconst label: string = '한글😀'; const n: number = 1; throw new Error(label);\n",
        &[],
    );
    let (lead, fix, calls) = parts(&error);
    assert_eq!(lead, "/a.css.ts:2:57: JS execution error: Error: 한글😀");
    assert!(fix.starts_with("remove or guard"), "{fix}");
    assert_eq!(calls, "    at <main> (/a.css.ts:2:57)");
}

#[test]
#[serial]
fn lines_ending_with_carriage_returns_are_counted_once() {
    let error = error_of(
        "/a.css.ts",
        "import { createVar } from '@devup-ui/react';\r\n\r\nthrow new Error('x');\r\n",
        &[],
    );
    assert!(
        error.starts_with("/a.css.ts:3:7: JS execution error: Error: x. Fix: "),
        "{error}"
    );
}

#[test]
#[serial]
fn rewritten_imports_do_not_shift_what_follows() {
    let aliases = HashMap::from([(
        "@vanilla-extract/css".to_string(),
        ImportAlias::NamedToNamed,
    )]);
    let source = "import { style } from '@vanilla-extract/css'; throw new Error('x');\n";
    let aliased = transform_import_aliases_with_edits(source, "/a.css.ts", PACKAGE, &aliases);
    assert_ne!(aliased.code, source);
    let edits: Vec<&[Edit]> = vec![aliased.edits.as_slice()];
    reset_file_map();
    let error = execute_located(
        Stylesheet {
            filename: "/a.css.ts",
            code: &aliased.code,
            source,
            edits: &edits,
        },
        &ExtractOption::default(),
        None,
    )
    .err()
    .unwrap_or_default();
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/a.css.ts:1:53: JS execution error: Error: x");
    assert_eq!(calls, "    at <main> (/a.css.ts:1:53)");
}

#[test]
#[serial]
fn an_imported_module_is_told_by_its_own_file() {
    let error = error_of(
        "/a.css.ts",
        "import { createVar } from '@devup-ui/react';\nimport { color } from './b';\nexport const token = createVar(color);\n",
        &[(
            "/b.ts",
            "const label: string = 'helper boom';\n\nfunction fail(): string {\n  throw new Error(label);\n}\nexport const color: string = fail();\n",
        )],
    );
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/b.ts:4:9: JS execution error: Error: helper boom");
    assert_eq!(calls, "    at fail (/b.ts:4:9)\n    at <main> (/b.ts:6:34)");
}

#[test]
#[serial]
fn code_after_a_replaced_import_keeps_its_place() {
    let error = error_of(
        "/a.css.ts",
        "import { out } from './b';\nexport const x = out;\n",
        &[
            (
                "/b.ts",
                "import { f } from './c';\nexport const b = 'blue';\nexport const out = f();\n",
            ),
            (
                "/c.ts",
                "import { b } from './b';\nexport const f = () => {\n  const v = b;\n  throw new Error(v);\n};\n",
            ),
        ],
    );
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/c.ts:4:9: JS execution error: Error: blue");
    assert_eq!(calls, "    at f (/c.ts:4:9)\n    at <main> (/b.ts:3:21)");
}

#[test]
#[serial]
fn a_default_export_keeps_its_place() {
    let error = error_of(
        "/a.css.ts",
        "export const label: string = 'x';\nexport default (() => {\n  throw new Error(label);\n})();\n",
        &[],
    );
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/a.css.ts:3:9: JS execution error: Error: x");
    assert_eq!(
        calls,
        "    at <main> (/a.css.ts:3:9)\n    at <main> (/a.css.ts:4:3)"
    );
}

#[test]
#[serial]
fn an_early_read_in_an_import_cycle_points_at_the_read() {
    let error = error_of(
        "/a.css.ts",
        "import { createVar } from '@devup-ui/react';\nimport { color } from './b';\nexport const token = createVar(color);\n",
        &[
            (
                "/b.ts",
                "import { other } from './c';\nexport const color = other;\n",
            ),
            (
                "/c.ts",
                "import { color } from './b';\nexport const other = color;\n",
            ),
        ],
    );
    let (lead, fix, calls) = parts(&error);
    assert_eq!(
        lead,
        "/c.ts:2:22: JS execution error: ReferenceError: Cannot access 'color' of '/b.ts' before its initialization: it is part of an import cycle"
    );
    assert!(fix.starts_with("break the import cycle"), "{fix}");
    assert_eq!(calls, "    at <main> (/c.ts:2:22)");
}

#[test]
#[serial]
fn a_throw_of_what_is_not_an_error_and_calls_the_build_gives_are_told() {
    let error = error_of("/a.css.ts", "throw 'text';\n", &[]);
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/a.css.ts:1:7: JS execution error: \"text\"");
    assert_eq!(calls, "    at <main> (/a.css.ts:1:7)");

    let error = error_of(
        "/a.css.ts",
        "[1].map(() => { throw new Error('in map'); });\n",
        &[],
    );
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/a.css.ts:1:23: JS execution error: Error: in map");
    assert_eq!(
        calls,
        "    at <main> (/a.css.ts:1:23)\n    at map (native)\n    at <main> (/a.css.ts:1:8)"
    );
}

#[test]
#[serial]
fn duplicate_lexical_declarations_are_located_before_execution() {
    let error = error_of("/a.css.ts", "let a = 1;\nlet a = 2;\n", &[]);
    let (lead, fix, calls) = parts(&error);
    assert!(
        lead.starts_with("/a.css.ts:1:5: JS execution error: SyntaxError:"),
        "{error}"
    );
    assert_eq!(fix, "correct the syntax at this location");
    assert_eq!(calls, "");
}

#[test]
#[serial]
fn retained_imported_functions_keep_their_original_frame() {
    let error = error_of(
        "/a.css.ts",
        "import { fail } from './d.css';\nexport const x = fail();\n",
        &[(
            "/d.css.ts",
            "import { style } from '@devup-ui/react';\nexport const s = style({ color: 'red' });\nexport const fail = () => { throw new Error('late'); };\n",
        )],
    );
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/d.css.ts:3:35: JS execution error: Error: late");
    assert_eq!(
        calls,
        "    at fail (/d.css.ts:3:35)\n    at <main> (/a.css.ts:2:22)"
    );
}

#[test]
#[serial]
fn an_imported_stylesheet_is_told_by_its_own_file() {
    let error = error_of(
        "/a.css.ts",
        "import { d } from './d.css';\nexport const x = d;\n",
        &[(
            "/d.css.ts",
            "import { style } from '@devup-ui/react';\n\nthrow new Error('nested');\nexport const d = style({});\n",
        )],
    );
    let (lead, _, calls) = parts(&error);
    assert_eq!(lead, "/d.css.ts:3:7: JS execution error: Error: nested");
    assert_eq!(calls, "    at <main> (/d.css.ts:3:7)");
}

#[test]
#[serial]
fn an_import_that_cannot_be_loaded_is_told_where_it_is_written() {
    reset_file_map();
    let code = "import { b } from './b';\nexport const x = b;\n";
    let without_resolver = execute_located(
        Stylesheet {
            filename: "/a.css.ts",
            code,
            source: code,
            edits: &[],
        },
        &ExtractOption::default(),
        None,
    )
    .err()
    .unwrap_or_default();
    assert!(
        without_resolver
            .starts_with("/a.css.ts:1:19: Cannot load './b' without a module resolver. Fix: "),
        "{without_resolver}"
    );

    for (code, expected) in [
        (
            "import { b } from './missing';\nexport const x = b;\n",
            "/a.css.ts:1:19: Cannot resolve './missing' from '/a.css.ts'. Fix: ",
        ),
        (
            "export { x } from './missing';\n",
            "/a.css.ts:1:19: Cannot resolve './missing' from '/a.css.ts'. Fix: ",
        ),
        (
            "export * from './missing';\n",
            "/a.css.ts:1:15: Cannot resolve './missing' from '/a.css.ts'. Fix: ",
        ),
        (
            "import { b } from './m';\nexport const x = b;\n",
            "/m.ts:1:19: Cannot resolve './nowhere' from '/m.ts'. Fix: ",
        ),
    ] {
        let error = error_of(
            "/a.css.ts",
            code,
            &[(
                "/m.ts",
                "import { z } from './nowhere';\nexport const b = z;\n",
            )],
        );
        assert!(error.starts_with(expected), "{error}");
    }

    let required = error_of(
        "/a.css.js",
        "const { color } = require('./missing');\nexports.card = color;\n",
        &[],
    );
    assert!(
        required.starts_with("/a.css.js:1:27: Cannot resolve './missing' from '/a.css.js'. Fix: "),
        "{required}"
    );
}
