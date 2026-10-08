use super::*;
use crate::{SourceMapMode, code_extract_internal_impl, export_sheet_internal};
use std::{cell::RefCell, rc::Rc};

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

#[test]
#[serial_test::serial]
fn extraction_when_imported_constant_uses_shadowed_require_never_requests_that_module()
-> Result<(), String> {
    // Given
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './helper';\nconst x = <Box bg={color} />;";
    let state = Rc::new(RefCell::new(ResolverState::new("entry.tsx", source)));
    let recorded = Rc::clone(&state);
    let requested = Rc::new(RefCell::new(Vec::new()));
    let requests = Rc::clone(&requested);
    let resolver = move |specifier: &str, importer: &str| {
        requests
            .borrow_mut()
            .push((importer.to_string(), specifier.to_string()));
        let request = recorded.borrow().request(specifier, importer)?;
        match (request.importer(), request.specifier()) {
            ("entry.tsx", "./helper") => {
                let module = ResolvedModule {
                    path: "helper.ts".to_string(),
                    code: "const require = () => ({color: 'red'}); const {color} = require('./shadowed'); exports.color = color;".to_string(),
                };
                recorded.borrow_mut().cache(&module, &request);
                Some(module)
            }
            other => panic!("unexpected module reference: {other:?}"),
        }
    };
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
    result?;
    assert_eq!(state.borrow().check(), Ok(()));
    assert!(
        requested
            .borrow()
            .iter()
            .any(|(importer, specifier)| importer == "entry.tsx" && specifier == "./helper")
    );
    assert!(
        !requested
            .borrow()
            .iter()
            .any(|(_, specifier)| specifier == "./shadowed")
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn publication_when_imported_constant_uses_failing_unbound_require_retains_ingress()
-> Result<(), String> {
    // Given
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './helper';\nconst x = <Box bg={color} />;";
    let state = Rc::new(RefCell::new(ResolverState::new("entry.tsx", source)));
    let recorded = Rc::clone(&state);
    let requested = Rc::new(RefCell::new(Vec::new()));
    let requests = Rc::clone(&requested);
    let resolver = move |specifier: &str, importer: &str| {
        let request = recorded.borrow().request(specifier, importer)?;
        requests.borrow_mut().push((
            request.importer().to_string(),
            request.specifier().to_string(),
        ));
        match (request.importer(), request.specifier()) {
            ("entry.tsx", "./helper") => {
                let module = ResolvedModule {
                    path: "helper.ts".to_string(),
                    code: "const {color} = require('./unbound'); exports.color = color;"
                        .to_string(),
                };
                recorded.borrow_mut().cache(&module, &request);
                Some(module)
            }
            ("helper.ts", "./unbound") => {
                recorded
                    .borrow_mut()
                    .record(&request, "unbound resolver failure");
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
    let error = result
        .err()
        .ok_or("unbound require unexpectedly succeeded")?;
    assert!(
        requested
            .borrow()
            .iter()
            .any(|(importer, specifier)| importer == "helper.ts" && specifier == "./unbound")
    );
    assert!(error.starts_with("helper.ts:1:25: module resolver failed for `./unbound` from `helper.ts`: unbound resolver failure"), "{error}");
    assert!(error.contains("Fix: repair the module resolver"), "{error}");
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}

#[test]
fn diagnostic_when_exploration_has_no_semantic_reference_retains_original_ingress()
-> Result<(), String> {
    // Given
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './helper';\nconst x = <Box bg={color} />;";
    let mut state = ResolverState::new("entry.tsx", source);
    let ingress = state
        .request("./helper", "entry.tsx")
        .ok_or("missing helper ingress")?;
    state.cache(
        &ResolvedModule {
            path: "helper.ts".to_string(),
            code: "exports.color = 'red';".to_string(),
        },
        &ingress,
    );
    let request = state
        .request("./exploratory", "helper.ts")
        .ok_or("missing exploratory ingress")?;
    // When
    state.record(&request, "reference-free exploration failed");
    // Then
    let error = state.check().err().ok_or("exploration failure was lost")?;
    assert!(error.starts_with("entry.tsx:2:23: module resolver failed for `./exploratory` from `helper.ts`: reference-free exploration failed"), "{error}");
    assert!(error.contains("Fix: repair the module resolver"), "{error}");
    Ok(())
}
