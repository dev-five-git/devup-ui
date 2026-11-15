use std::collections::BTreeSet;

use rstest::rstest;
use serial_test::serial;

use crate::{TestStateGuard, import_sheet_json_internal, with_style_sheet};

#[rstest]
#[case::persisted_frozen(r#"{"atom_plan":["old.tsx"],"atomNamingVersion":2}"#, Some(vec!["old.tsx"]), Some(vec!["old.tsx"]))]
#[case::persisted_unfrozen(r#"{"atomNamingVersion":2}"#, None, None)]
#[case::explicit_null(r#"{"atom_plan":["old.tsx"],"currentAtomPlan":null}"#, Some(vec!["old.tsx"]), None)]
#[case::explicit_empty(r#"{"currentAtomPlan":[]}"#, None, Some(vec![]))]
#[case::independent_plans(r#"{"atom_plan":["old.tsx"],"currentAtomPlan":["new.tsx"]}"#, Some(vec!["old.tsx"]), Some(vec!["new.tsx"]))]
#[serial]
fn checkpoint_pair_import_distinguishes_persisted_missing_from_explicit_global_plan(
    #[case] wire: &str,
    #[case] sheet_plan: Option<Vec<&str>>,
    #[case] global_plan: Option<Vec<&str>>,
) -> Result<(), serde_json::Error> {
    // Given: shipped flat sheet data or a transport with explicit current-global metadata.
    let _scope = TestStateGuard::new();
    let expected_sheet = sheet_plan.map(|files| {
        files
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
    });
    let expected_global = global_plan.map(|files| {
        files
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
    });
    // When: the native JSON boundary parses and installs the checkpoint.
    import_sheet_json_internal(wire)?;
    // Then: only missing metadata falls back to the shipped atom_plan contract.
    assert_eq!(
        with_style_sheet(|sheet| sheet.atom_plan.clone()),
        expected_sheet
    );
    assert_eq!(css::atom_hoist::atom_plan(), expected_global);
    Ok(())
}

#[test]
#[serial]
fn checkpoint_pair_invalid_metadata_leaves_both_live_plans_unchanged() {
    // Given: a genuine nonempty pair and malformed new metadata.
    let _scope = TestStateGuard::new();
    let before = crate::checkpoint_pair_tests::frozen_pair_fixture();
    let global = css::atom_hoist::atom_plan();
    // When: decoding fails before installation.
    let result = import_sheet_json_internal(r#"{"currentAtomPlan":123}"#);
    // Then: neither authority nor emission bytes were replaced.
    assert!(result.is_err());
    assert_eq!(with_style_sheet(|sheet| sheet.atom_plan.clone()), None);
    assert_eq!(css::atom_hoist::atom_plan(), global);
    assert_eq!(crate::configuration::css_internal(None, false), before);
}
