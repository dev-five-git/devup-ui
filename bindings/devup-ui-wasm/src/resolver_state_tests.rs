use super::*;
use crate::{SourceMapMode, code_extract_internal_impl, export_sheet_internal};
use extractor::ResolvedModule;
use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

#[test]
fn reservation_roots_keep_their_original_diagnostic_sites() -> Result<(), String> {
    // Given
    let mut state = ResolverState::default();
    assert!(state.request("./shared", "first.ts").is_none());
    assert!(state.request("./shared", "second.ts").is_none());
    state.add_source("first.ts", "import './shared';");
    state.add_source("second.ts", "\nimport './shared';");

    // When
    for filename in ["second.ts", "first.ts"] {
        let request = state
            .request("./shared", filename)
            .ok_or("missing reservation root reference")?;
        state.record(&request, "reservation resolver failure");
    }

    // Then
    assert!(state.request("./shared", "first.ts").is_some());
    assert!(state.request("./shared", "second.ts").is_some());
    let error = state.check().err().ok_or("missing resolver diagnostics")?;
    let sites = error
        .lines()
        .filter(|line| line.contains("module resolver failed"))
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 2);
    assert!(sites[0].starts_with("first.ts:1:8:"), "{error}");
    assert!(sites[1].starts_with("second.ts:2:8:"), "{error}");
    Ok(())
}

#[test]
fn diagnostic_when_resolver_fails_in_cached_imported_typescript() -> Result<(), String> {
    // Given
    let mut state = ResolverState::new("entry.tsx", "import './barrel';");
    let ingress = state
        .request("./barrel", "entry.tsx")
        .ok_or("missing ingress reference")?;
    state.cache(
        &ResolvedModule {
            path: "barrel.ts".to_string(),
            code: "type Color = string;\nexport {\n color\n} from './color';".to_string(),
        },
        &ingress,
    );
    let request = state
        .request("./color", "barrel.ts")
        .ok_or("missing imported reference")?;
    // When
    state.record(&request, "permission denied: /original/path");
    // Then
    let error = state.check().err().unwrap_or_default();
    assert!(error.starts_with("barrel.ts:4:8:"));
    assert!(error.contains("permission denied: /original/path"));
    assert!(error.contains("Fix: repair the module resolver"));
    Ok(())
}

#[test]
fn diagnostic_when_specifier_has_multiple_original_sites() -> Result<(), String> {
    // Given
    let mut state = ResolverState::new("entry.ts", "import './color';\nexport * from './color';");
    let request = state
        .request("./color", "entry.ts")
        .ok_or("missing original reference")?;
    // When
    state.record(&request, "broken resolver");
    // Then
    let error = state.check().err().unwrap_or_default();
    let diagnostics = error
        .lines()
        .filter(|line| line.starts_with("entry.ts:"))
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics[0].starts_with("entry.ts:1:8: module resolver failed"));
    assert!(diagnostics[1].starts_with("entry.ts:2:15: module resolver failed"));
    assert!(
        diagnostics
            .iter()
            .all(|line| line.ends_with("broken resolver"))
    );
    Ok(())
}

#[test]
fn diagnostic_when_repeated_resolution_fails_is_deduplicated() -> Result<(), String> {
    // Given
    let mut state = ResolverState::new("entry.ts", "import './color';");
    let request = state
        .request("./color", "entry.ts")
        .ok_or("missing original reference")?;
    state.record(&request, "broken resolver");
    let first = state.check();
    // When
    state.record(&request, "broken resolver");
    // Then
    assert_eq!(state.check(), first);
    Ok(())
}

#[test]
#[serial_test::serial]
fn publication_when_exploratory_resolution_fails_is_rejected() -> Result<(), String> {
    // Given
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './color';\nconst x = <Box bg='red' />;";
    let state = Rc::new(RefCell::new(ResolverState::new("entry.tsx", source)));
    let recorded = Rc::clone(&state);
    let resolver = move |specifier: &str, importer: &str| {
        let request = recorded.borrow().request(specifier, importer)?;
        recorded
            .borrow_mut()
            .record(&request, "exploratory resolver failure");
        None
    };
    let before = export_sheet_internal()?;
    // When
    let result = code_extract_internal_impl(
        "entry.tsx",
        source,
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        SourceMapMode::Generate,
        Some(&resolver),
        Some(&state),
    );
    // Then
    let error = result.err().unwrap_or_default();
    assert!(error.starts_with("entry.tsx:2:23:"));
    assert!(error.contains("exploratory resolver failure"));
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}

#[test]
fn clean_extraction_state_when_another_extraction_failed() -> Result<(), String> {
    // Given
    let mut failed = ResolverState::new("entry.ts", "import './color';");
    let request = failed
        .request("./color", "entry.ts")
        .ok_or("missing original reference")?;
    failed.record(&request, "broken resolver");
    // When
    let clean = ResolverState::new("entry.ts", "import './color';");
    // Then
    assert_eq!(clean.check(), Ok(()));
    Ok(())
}

#[rstest::rstest]
#[case::global("const {color} = require('./missing'); exports.color = color;", true)]
#[case::shadowed(
    "const require = () => ({color: 'red'}); const {color} = require('./shadowed'); exports.color = color;",
    false
)]
#[serial_test::serial]
fn require_exploration_keeps_real_site_and_shadowed_value(
    #[case] helper: &'static str,
    #[case] global: bool,
) -> Result<(), String> {
    let _scope = crate::TestStateGuard::new();
    crate::state::reset_state_internal();
    // Given
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './helper';\nconst x = <Box bg={color} />;";
    let state = Rc::new(RefCell::new(ResolverState::new("entry.tsx", source)));
    let recorded = Rc::clone(&state);
    let resolver = move |specifier: &str, importer: &str| {
        let request = recorded.borrow().request(specifier, importer)?;
        match (request.importer(), request.specifier()) {
            ("entry.tsx", "./helper") => {
                let module = ResolvedModule {
                    path: "helper.ts".to_string(),
                    code: helper.to_string(),
                };
                recorded.borrow_mut().cache(&module, &request);
                Some(module)
            }
            ("helper.ts", "./shadowed") => panic!("a shadowed require must never request a module"),
            ("helper.ts", "./missing") => {
                recorded
                    .borrow_mut()
                    .record(&request, "missing resolver failure");
                None
            }
            other => panic!("unexpected module reference: {other:?}"),
        }
    };
    let before = export_sheet_internal()?;
    // When
    let result = code_extract_internal_impl(
        "entry.tsx",
        source,
        "@devup-ui/react",
        "df".to_string(),
        true,
        false,
        false,
        HashMap::new(),
        SourceMapMode::Generate,
        Some(&resolver),
        Some(&state),
    );
    // Then
    if global {
        let error = result
            .err()
            .ok_or("global require must preserve the resolver failure")?;
        assert!(error.starts_with("helper.ts:1:25: module resolver failed for `./missing` from `helper.ts`: missing resolver failure"), "{error}");
        assert_eq!(export_sheet_internal()?, before);
    } else {
        assert_eq!(
            result?.code(),
            "import \"df/devup-ui.css\";\nimport { color } from \"./helper\";\nconst x = <div className=\"a\" style={{ \"--b\": color }} />;\n"
        );
    }
    Ok(())
}

#[test]
fn cached_request_when_no_semantic_site_retains_ingress() -> Result<(), String> {
    // Given
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './helper';\nconst x = <Box bg={color} />;";
    let mut state = ResolverState::new("entry.tsx", source);
    let ingress = state.request("./helper", "entry.tsx").ok_or("no ingress")?;
    state.cache(
        &ResolvedModule {
            path: "helper.ts".to_string(),
            code: "const target = './missing'; require(target);".to_string(),
        },
        &ingress,
    );
    let request = state
        .request("./missing", "helper.ts")
        .ok_or("no request")?;
    // When
    state.record(&request, "cached resolver failure");
    // Then
    let error = state.check().err().ok_or("no resolver diagnostic")?;
    assert!(error.starts_with("entry.tsx:2:23: module resolver failed for `./missing` from `helper.ts`: cached resolver failure"), "{error}");
    Ok(())
}
