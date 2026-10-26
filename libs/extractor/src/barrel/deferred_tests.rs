use std::cell::RefCell;

use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ModuleResolution, ResolvedModule, extract_with_source_policy};

#[rstest]
#[case("./doc.mdx", "./page.tsx", "./doc.mdx")]
#[case("./forward.js", "/forward.js", "./doc.mdx")]
#[serial]
fn deferred_mdx_preserves_root_and_cut_when_source_is_unchanged(
    #[case] request: &str,
    #[case] cut_importer: &str,
    #[case] cut_request: &str,
) {
    // Given an unchanged render-only module and an ordinary forwarding barrel.
    let _state = crate::test_state::TestStateGuard::new();
    let code =
        format!("const label='한😀'; import Doc from '{request}'; export const view=<Doc/>;");
    let resolver = |source: &str, _: &str| {
        (source == "./forward.js").then(|| {
            ModuleResolution::Resolved(ResolvedModule {
                path: "/forward.js".to_string(),
                code: "export {default} from './doc.mdx'".to_string(),
                source_type: None,
            })
        })
    };
    // When the existing extraction defers a semantic default read.
    let Ok(output) = extract_with_source_policy(
        "./page.tsx",
        &code,
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("render-only default extraction succeeds");
    };
    // Then it retains the authored UTF-16 location and both identities without edits.
    assert_eq!(output.code, code);
    let [record] = output.deferred_defaults.as_slice() else {
        panic!("one obligation required")
    };
    assert_eq!(record.extraction, "./page.tsx");
    assert_eq!(record.request, request);
    assert_eq!(record.exports, ["default"]);
    assert_eq!(record.line, 1);
    assert_eq!(
        record.column,
        "const label='한😀'; import ".encode_utf16().count() + 1
    );
    assert_eq!(record.cut_importer, cut_importer);
    assert_eq!(record.cut_request, cut_request);
    assert_eq!(record.cut_export, "default");
}

#[rstest]
#[case("import Doc from './doc.mdx'; export const n=1;", true)]
#[case("import * as Docs from './doc.mdx'; export const n=1;", true)]
#[case("import Doc from './doc.mdx'; export const view=<Doc/>;", false)]
#[serial]
fn deferred_mdx_requires_value_demand_and_explicit_policy(
    #[case] code: &str,
    #[case] enabled: bool,
) {
    // Given unused or legacy input and a resolver with no source.
    let _state = crate::test_state::TestStateGuard::new();
    let policy = if enabled {
        vec![".mdx".to_string()]
    } else {
        vec![]
    };
    // When extraction runs without a supported deferred demand.
    let Ok(output) = extract_with_source_policy(
        "/page.tsx",
        code,
        ExtractOption::default(),
        false,
        Some(&|_, _| None),
        None,
        &policy,
    ) else {
        panic!("unused or legacy input extracts without obligations");
    };
    // Then no native acceptance obligation is manufactured.
    assert_eq!(output.deferred_defaults, vec![]);
}

#[rstest]
#[case("\n", 2)]
#[case("\r\n", 2)]
#[case("\r", 2)]
#[case("\u{2028}", 2)]
#[case("\u{2029}", 2)]
#[case("\r\n\u{2028}\r", 4)]
#[serial]
fn deferred_mdx_locates_legal_line_terminators(#[case] newline: &str, #[case] line: usize) {
    // Given legal JS newlines and UTF-16 characters before the authored binding.
    let _state = crate::test_state::TestStateGuard::new();
    let prefix = "const after='한😀'; import ";
    let code = format!(
        "const before='한😀';{newline}{prefix}Doc from './doc.mdx'; export const view=<Doc/>;"
    );
    // When the actual extraction records its deferred root.
    let Ok(output) = extract_with_source_policy(
        "/page.tsx",
        &code,
        ExtractOption::default(),
        false,
        Some(&|_, _| None),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("legal JavaScript line terminators extract");
    };
    // Then every line terminator resets the UTF-16 column and CRLF counts once.
    let [record] = output.deferred_defaults.as_slice() else {
        panic!("one obligation required")
    };
    assert_eq!(record.line, line);
    assert_eq!(record.column, prefix.encode_utf16().count() + 1);
}

#[rstest]
#[case("import React from 'react';", "<Doc/>", true)]
#[case("import * as React from 'react';", "<Doc/>", true)]
#[case("import {jsx as make} from 'react/jsx-runtime';", "make(Doc,{})", true)]
#[case(
    "import {jsxs as make} from 'react/jsx-runtime';",
    "make(Doc,{})",
    true
)]
#[case(
    "import {jsxDEV as make} from 'react/jsx-dev-runtime';",
    "make(Doc,{})",
    true
)]
#[case("import {createElement as make} from 'react';", "make(Doc,{})", true)]
#[case("import React from 'react';", "React.createElement(Doc,{})", false)]
#[case(
    "import * as React from 'react';",
    "React.createElement(Doc,{})",
    false
)]
#[case("import React from 'react';", "React(Doc,{})", false)]
#[case(
    "import * as React from 'react/jsx-runtime';",
    "React.jsx(Doc,{})",
    false
)]
#[serial]
fn deferred_mdx_requires_a_proven_render_or_named_factory(
    #[case] react_import: &str,
    #[case] render: &str,
    #[case] deferred: bool,
) {
    // Given real React import shapes and an authoritative resolver with no source.
    let _state = crate::test_state::TestStateGuard::new();
    let reads = RefCell::new(Vec::new());
    let resolver = |source: &str, _: &str| {
        reads.borrow_mut().push(source.to_string());
        None
    };
    let code = format!(
        "import {{Box}} from '@devup-ui/react'; {react_import} import Doc from './doc.mdx'; export const view=<Box bg='red'>{{{render}}}</Box>;"
    );
    // When extraction classifies the component argument through its actual binding.
    let Ok(output) = extract_with_source_policy(
        "/page.tsx",
        &code,
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("React demand controls extract alongside own styles");
    };
    // Then only proven render demand skips preparation and creates native debt.
    assert_eq!(output.deferred_defaults.len(), usize::from(deferred));
    assert_eq!(
        reads.borrow().iter().any(|source| source == "./doc.mdx"),
        !deferred
    );
    assert_ne!(output.styles.len(), 0);
}

#[test]
#[serial]
fn deferred_mdx_preserves_unsupported_import_attributes() {
    // Given an attributed MDX import that is used only as a JSX component.
    let _state = crate::test_state::TestStateGuard::new();
    let reads = RefCell::new(Vec::new());
    let resolver = |source: &str, _: &str| {
        reads.borrow_mut().push(source.to_string());
        None
    };
    let code = "import Doc from './doc.mdx' with {type:'json'}; export const view=<Doc/>;";
    // When extraction defers the default rather than reading raw Markdown.
    let Ok(output) = extract_with_source_policy(
        "/page.tsx",
        code,
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("attributed render demand retains its admission obligation");
    };
    // Then the native obligation retains the unsupported condition and root identity.
    let [record] = output.deferred_defaults.as_slice() else {
        panic!("one attributed obligation required");
    };
    assert!(record.attributes);
    assert_eq!(record.extraction, "/page.tsx");
    assert_eq!(record.request, "./doc.mdx");
    assert_eq!(record.exports, ["default"]);
    assert_eq!(record.cut_importer, "/page.tsx");
    assert_eq!(record.cut_request, "./doc.mdx");
    assert_eq!(output.code, code);
    assert!(!reads.borrow().iter().any(|source| source == "./doc.mdx"));
}
