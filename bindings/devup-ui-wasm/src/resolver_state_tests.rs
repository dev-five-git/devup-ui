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
fn diagnostic_when_imported_constant_exploration_uses_global_require_locates_its_source()
-> Result<(), String> {
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
                    code: "const {color} = require('./missing'); exports.color = color;"
                        .to_string(),
                };
                recorded.borrow_mut().cache(&module, &request);
                Some(module)
            }
            ("helper.ts", "./missing") => {
                recorded
                    .borrow_mut()
                    .record(&request, "global resolver failure");
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
    let error = result.err().unwrap_or_default();
    assert!(error.starts_with("helper.ts:1:25: module resolver failed for `./missing` from `helper.ts`: global resolver failure"), "{error}");
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}

#[test]
fn diagnostic_when_a_request_has_no_source_site_retains_the_entry_ingress() -> Result<(), String> {
    // Given
    let mut state = ResolverState::new(
        "entry.tsx",
        "import { Box } from '@devup-ui/react';\nimport { color } from './helper';",
    );
    let ingress = state
        .request("./helper", "entry.tsx")
        .ok_or("missing ingress")?;
    state.cache(
        &ResolvedModule {
            path: "helper.ts".to_string(),
            code: "export const color = 'red';".to_string(),
        },
        &ingress,
    );
    let request = state
        .request("./unlocated", "helper.ts")
        .ok_or("missing fallback request")?;
    // When
    state.record(&request, "fallback resolver failure");
    // Then
    let error = state.check().err().ok_or("missing failure")?;
    assert!(error.starts_with("entry.tsx:2:23: module resolver failed for `./unlocated` from `helper.ts`: fallback resolver failure"), "{error}");
    Ok(())
}

#[test]
#[serial_test::serial]
fn imported_constant_when_require_is_shadowed_stays_dynamic_without_resolving_its_argument()
-> Result<(), String> {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::debug::set_debug(false);
    let source = "import { Box } from '@devup-ui/react';\nimport { color } from './helper';\nconst x = <Box bg={color} />;";
    let state = Rc::new(RefCell::new(ResolverState::new("entry.tsx", source)));
    let recorded = Rc::clone(&state);
    let resolver = move |specifier: &str, importer: &str| {
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
            ("helper.ts", "./shadowed") => panic!("shadowed require reached the resolver"),
            other => panic!("unexpected module reference: {other:?}"),
        }
    };
    // When
    let output = code_extract_internal_impl(
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
    )?;
    // Then
    assert_eq!(
        output.code(),
        "import \"df/devup-ui.css\";\nimport { color } from \"./helper\";\nconst x = <div className=\"a\" style={{ \"--b\": color }} />;\n"
    );
    assert_eq!(state.borrow().check(), Ok(()));
    Ok(())
}
