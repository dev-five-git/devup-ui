use std::cell::RefCell;

use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ModuleResolution, ResolvedModule, extract_with_source_policy};

#[path = "coverage_demand_tests.rs"]
mod coverage_tests;

#[rstest]
#[case("import Doc from './doc.mdx'; export const view=<Box bg='red'><Doc/></Box>;")]
#[case(
    "import Doc from './doc.mdx'; const Alias=Doc; export const view=<Box bg='red'><Alias/></Box>;"
)]
#[case(
    "import Doc from './doc.mdx'; function f(Doc){return Doc} export const view=<Box bg='red'><Doc/></Box>;"
)]
#[case("import {tone} from './doc.mdx'; export const view=<Box bg='red'/>;")]
#[case("import * as Docs from './doc.mdx'; export const view=<Box bg='red'/>;")]
#[case(
    "import Doc from './doc.mdx'; import {jsx as create} from 'react/jsx-runtime'; export const view=<Box bg='red'>{create(Doc,{})}</Box>;"
)]
#[serial]
fn opaque_mdx_render_demand_never_reads_markdown(#[case] body: &str) {
    // Given an actual extraction with a compiler-certified MDX default policy.
    let _state = crate::test_state::TestStateGuard::new();
    let reads = RefCell::new(Vec::new());
    let resolver = |source: &str, _: &str| {
        reads.borrow_mut().push(source.to_string());
        None
    };
    let code = format!("import {{Box}} from '@devup-ui/react'; {body}");
    // When own styles are extracted alongside a runtime-only default component.
    let Ok(output) = extract_with_source_policy(
        "/app.tsx",
        &code,
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("render-only MDX demand extracts without reading Markdown");
    };
    // Then no Markdown source was requested and own styling still compiles.
    assert_eq!(
        reads
            .borrow()
            .iter()
            .filter(|source| {
                std::path::Path::new(source)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("mdx"))
            })
            .count(),
        0
    );
    assert!(output.code.contains("<div className="), "{}", output.code);
    assert_ne!(output.styles.len(), 0);
}

#[rstest]
#[case("import {Widget} from './doc.mdx'; export const view=<Box bg='red'><Widget/></Box>;")]
#[case("import {tone} from './doc.mdx'; export const view=<Box bg={tone}/>;")]
#[case("import Doc from './doc.mdx'; export const view=<Box bg={Doc}/>;")]
#[case(
    "import Doc from './doc.mdx'; const Alias=Doc; export {Alias}; export const view=<Box bg='red'/>;"
)]
#[serial]
fn opaque_mdx_value_demand_still_reaches_the_authoritative_resolver(#[case] body: &str) {
    // Given a resolver that records real attempted reads but supplies no Markdown.
    let _state = crate::test_state::TestStateGuard::new();
    let reads = RefCell::new(Vec::new());
    let resolver = |source: &str, _: &str| {
        reads.borrow_mut().push(source.to_string());
        None
    };
    let code = format!("import {{Box}} from '@devup-ui/react'; {body}");
    // When a binding is needed as a named primitive, style value or exported value.
    let _output = extract_with_source_policy(
        "/app.tsx",
        &code,
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    );
    // Then default-render opacity cannot suppress that source-preparation boundary.
    assert!(reads.borrow().iter().any(|source| source == "./doc.mdx"));
}

#[test]
#[serial]
fn opaque_mdx_policy_preserves_an_ordinary_default_barrel_and_prop_free_primitive() {
    // Given ordinary JS forwarding a Devup default and an MDX default reexport.
    let _state = crate::test_state::TestStateGuard::new();
    let reads = RefCell::new(Vec::new());
    let resolver = |source: &str, _: &str| {
        reads.borrow_mut().push(source.to_string());
        let code = match source {
            "./ui.js" => "export {Box as default} from '@devup-ui/react';",
            "./docs.js" => "export {default} from './doc.mdx';",
            _ => return None,
        };
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: source.to_string(),
            code: code.to_string(),
            source_type: None,
        }))
    };
    // When both bindings are used as prop-free JSX components.
    let Ok(output) = extract_with_source_policy(
        "/app.tsx",
        "import Widget from './ui.js'; import Doc from './docs.js'; export const view=<Widget><Doc/></Widget>;",
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("ordinary default forwarding extracts");
    };
    // Then the real primitive compiles while the forwarded certified default stays runtime.
    assert!(output.code.contains("<div"), "{}", output.code);
    assert!(!output.code.contains("<Widget"));
    assert!(!reads.borrow().iter().any(|source| source == "./doc.mdx"));
    assert!(reads.borrow().iter().any(|source| source == "./ui.js"));
}

#[rstest]
#[case(
    "import {Widget} from './ui.js';",
    "export {Box as Widget} from '@devup-ui/react';",
    "Widget"
)]
#[case(
    "import {Widget as Alias} from './ui.js';",
    "export {Box as Widget} from '@devup-ui/react';",
    "Alias"
)]
#[case(
    "import * as UI from './ui.js';",
    "export {Box as Widget} from '@devup-ui/react';",
    "UI.Widget"
)]
#[case(
    "import {Box} from './ui.js';",
    "export * from '@devup-ui/react';",
    "Box"
)]
#[serial]
fn opaque_mdx_policy_preserves_ordinary_js_forwarding(
    #[case] import: &str,
    #[case] forwarded: &str,
    #[case] tag: &str,
) {
    // Given an ordinary named, alias, namespace or star forwarding path.
    let _state = crate::test_state::TestStateGuard::new();
    let resolver = |source: &str, _: &str| {
        assert_eq!(source, "./ui.js");
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "/ui.js".to_string(),
            code: forwarded.to_string(),
            source_type: None,
        }))
    };
    // When the MDX default policy is enabled for a prop-free forwarded primitive.
    let Ok(output) = extract_with_source_policy(
        "/app.tsx",
        &format!("{import} export const view=<{tag}/>;"),
        ExtractOption::default(),
        false,
        Some(&resolver),
        None,
        &[".mdx".to_string()],
    ) else {
        panic!("ordinary JavaScript forwarding extracts");
    };
    // Then the forwarding path still compiles to a native element, never a placeholder.
    assert!(output.code.contains("<div"), "{}", output.code);
    assert!(!output.code.contains(&format!("<{tag}")));
}
