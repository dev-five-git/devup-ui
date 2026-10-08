use serial_test::serial;

use super::{extracted, visit};

const ENTRIES: [(&str, &str); 14] = [
    ("import sx from '@stylexjs/stylex';", "sx"),
    ("import * as sx from '@stylexjs/stylex';", "sx"),
    (
        "import { positionTry as pt, viewTransitionClass as vt } from '@stylexjs/stylex';",
        "named",
    ),
    ("import * as sx from '@devup-ui/react/stylex';", "sx"),
    (
        "import { positionTry as pt, viewTransitionClass as vt } from '@devup-ui/react/stylex';",
        "named",
    ),
    ("import { stylex as sx } from '@devup-ui/react';", "sx"),
    ("import * as devup from '@devup-ui/react';", "devup.stylex"),
    ("const sx = require('@devup-ui/react/stylex');", "sx"),
    (
        "const { positionTry: pt, viewTransitionClass: vt } = require('@devup-ui/react/stylex');",
        "named",
    ),
    ("const { stylex: sx } = require('@devup-ui/react');", "sx"),
    ("const devup = require('@devup-ui/react');", "devup.stylex"),
    ("const sx = require('@stylexjs/stylex');", "sx"),
    ("const { default: sx } = require('@stylexjs/stylex');", "sx"),
    (
        "const { positionTry: pt, viewTransitionClass: vt } = require('@stylexjs/stylex');",
        "named",
    ),
];

#[test]
#[serial]
fn public_calls_are_removed_when_gate_and_extractor_share_entrypoints() {
    for (import, namespace) in ENTRIES {
        let (position, view) = if namespace == "named" {
            ("pt".to_string(), "vt".to_string())
        } else {
            (
                format!("{namespace}.positionTry"),
                format!("{namespace}.viewTransitionClass"),
            )
        };
        let code = format!(
            "{import}\nexport const p = {position}({{ width: 100 }});\nexport const v = {view}({{ old: {{ opacity: 0 }} }});"
        );
        assert!(
            crate::has_devup_ui_with(
                "public.ts",
                &code,
                "@devup-ui/react",
                &Default::default(),
                None
            ),
            "{code}"
        );
        let output = extracted(&code).unwrap_or_else(|error| panic!("{error}"));
        assert!(!output.contains("positionTry("), "{output}");
        assert!(!output.contains("viewTransitionClass("), "{output}");
        assert!(!output.contains("require("), "{output}");
        assert!(!output.contains("from \"@stylexjs/stylex\""), "{output}");
        assert!(
            !output.contains("from \"@devup-ui/react/stylex\""),
            "{output}"
        );
    }
}

#[test]
#[serial]
fn root_runtime_reads_survive_when_only_stylex_projection_is_compiled() {
    let code = "import * as devup from '@devup-ui/react';
export const theme = devup.getTheme();
export const name = devup.stylex.positionTry({ top: 0 });";
    let output = extracted(code).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains("devup.getTheme()"), "{output}");
    assert!(output.contains("import * as devup"), "{output}");
    assert!(!output.contains("devup.stylex"), "{output}");
}

#[test]
#[serial]
fn shadowed_api_and_loader_names_stay_user_code_when_bindings_differ() {
    let code = "import * as sx from '@devup-ui/react/stylex';
function user(sx, require) { const other = require('@devup-ui/react/stylex'); return [sx.positionTry({}), other.positionTry({})]; }
const name = sx.positionTry({ top: 0 });";
    let output = visit(code);
    assert_eq!(output.errors, Vec::<String>::new());
    assert!(
        output.code.contains("sx.positionTry({})"),
        "{}",
        output.code
    );
    assert!(
        output.code.contains("other.positionTry({})"),
        "{}",
        output.code
    );
}

#[test]
#[serial]
fn residual_escape_is_located_when_a_compiled_api_is_exported() {
    let cases = [
        "import * as sx from '@devup-ui/react/stylex';\nexport const f = sx.positionTry;",
        "import { positionTry as pt } from '@devup-ui/react/stylex';\nexport { pt };",
        "import { stylex as sx } from '@devup-ui/react';\nexport const api = sx;",
        "import * as devup from '@devup-ui/react';\nexport const api = devup.stylex;",
        "import * as sx from '@stylexjs/stylex';\nconst result = sx[method]({});",
        "import sx from '@devup-ui/react/stylex';\nconst result = sx.positionTry({});",
        "const sx = require('@devup-ui/react/stylex', options);\nconst result = sx.positionTry({});",
        "import * as stylex from '@stylexjs/stylex';\nexport const called = stylex({ a: 'b' });",
        "import * as stylex from '@stylexjs/stylex';\nexport const indirect = (0, stylex.defineVars)({ a: 'b' });",
    ];
    for code in cases {
        let error = extracted(code)
            .err()
            .unwrap_or_else(|| panic!("runtime API escape must fail"));
        assert!(error.contains("test.tsx:"), "{error}");
        assert!(error.contains("cannot use"), "{error}");
    }
}

#[test]
#[serial]
fn static_aliases_disappear_when_their_api_calls_are_consumed() {
    let code = "import * as devup from '@devup-ui/react';
const sx = devup.stylex;
const pt = sx.positionTry;
export const name = pt({ width: 10 });";
    let output = extracted(code).unwrap_or_else(|error| panic!("{error}"));
    assert!(!output.contains("import * as devup"), "{output}");
    assert!(!output.contains("devup.stylex"), "{output}");
    assert!(!output.contains("const sx"), "{output}");
    assert!(!output.contains("const pt"), "{output}");
}

#[test]
#[serial]
fn type_specifiers_survive_when_value_api_specifiers_are_consumed() {
    let code = "import { positionTry as pt, type PositionTryStyles } from '@devup-ui/react/stylex';
export const name = pt({ width: 10 });";
    let output = extracted(code).unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains("type PositionTryStyles"), "{output}");
    assert!(!output.contains("positionTry as pt"), "{output}");
}

#[test]
#[serial]
fn root_alias_chains_are_pruned_when_no_runtime_root_read_remains() {
    let code = "import * as devup from '@devup-ui/react'; const first = devup; const second = first; export const name = second.stylex.positionTry({ top: 0 });";
    let output = extracted(code).unwrap_or_else(|error| panic!("{error}"));
    assert!(!output.contains("import * as devup"), "{output}");
    assert!(!output.contains("= devup"), "{output}");
    assert!(!output.contains("const first"), "{output}");
    assert!(!output.contains("const second"), "{output}");
}
