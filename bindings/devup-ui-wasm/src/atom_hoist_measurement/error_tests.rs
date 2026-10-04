use super::{ExtractionError, build, fixture};
use serial_test::serial;

#[test]
#[serial]
fn build_reports_failing_file_when_fixture_contains_malformed_jsx() {
    // Given: a malformed first atom in the otherwise unchanged real fixture.
    let mut modules = fixture();
    modules[0].atoms[0].value = "\"".to_string();
    // When: extraction fails through the measurement build boundary.
    let error: ExtractionError = match build(&modules, true) {
        Ok(_) => panic!("malformed JSX must fail extraction"),
        Err(error) => error,
    };
    // Then: the typed error retains its failing file and parser context.
    assert_eq!(error.file, "src/shared/s00.tsx");
    assert_eq!(error.message, "Parser panicked");
    assert_eq!(error.to_string(), "src/shared/s00.tsx: Parser panicked");
    assert_eq!(css::atom_hoist::atom_hoist_threshold(), None);
    assert_eq!(css::file_routes::get_file_routes().len(), 0);

    // Given: a valid fixture after the failed build has released its state.
    let valid = fixture();
    // When: a subsequent build extracts that fixture without hoisting.
    let snapshot = build(&valid, false).unwrap_or_else(|error| panic!("{error}"));
    // Then: every emitted class loads the valid fixture's exact declaration.
    snapshot.assert_loaded_definitions(&valid);
    assert!(!snapshot.sheets[0].contains("{width:100000px}"));
    assert!(snapshot.sheets[1].contains("{width:100000px}"));
}
