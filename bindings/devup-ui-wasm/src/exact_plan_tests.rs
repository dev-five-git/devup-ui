use super::*;
use exact_test_support::{authority, compile, fixture};
use rstest::rstest;
use serial_test::serial;
use std::collections::{BTreeSet, HashSet};

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn real_parser_failure_restores_unfrozen_plan_when_routes_are_configured(#[case] eligible: bool) {
    // Given: configuration precedes capture and the real common extraction entry.
    fixture();
    set_atom_hoist(Some(2));
    import_file_routes_internal(HashMap::from([(
        "plan.tsx".into(),
        if eligible {
            HashSet::from([1, 2])
        } else {
            HashSet::from([1])
        },
    )]));
    assert_eq!(css::atom_hoist::atom_plan(), None);
    let before = authority();
    // When: extraction freezes a new plan before the parser fails.
    let error = compile(
        "broken-plan.tsx",
        "import {Box} from '@devup-ui/react';const x=<Box",
    )
    .err()
    .unwrap_or_else(|| panic!("broken plan fixture unexpectedly compiled"));
    // Then: ALL authority returns, including None rather than an empty plan.
    assert!(error.contains("Parser panicked"));
    assert_eq!(authority(), before);
    assert_eq!(cache_names::check(), Ok(()));
    // Independent positive control: unchanged configuration really freezes differently.
    css::atom_hoist::freeze_atom_plan();
    assert_eq!(
        css::atom_hoist::atom_plan(),
        Some(if eligible {
            BTreeSet::from(["plan.tsx".into()])
        } else {
            BTreeSet::new()
        })
    );
    reset_build_state_internal();
}
