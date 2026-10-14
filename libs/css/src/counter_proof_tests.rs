use std::collections::{BTreeSet, HashMap};

use rstest::rstest;
use serial_test::serial;

use crate::allocation_input::{
    AllocationFile, CapturedDelivery, LegacyDeclaration, LegacyInput, LegacyVariable, NameMode,
};
use crate::class_map::{Attempt, get_class_map, reset_class_map};
use crate::counter_names::{NameAddress, allocate_name, allocation_key, render_name};
use crate::counter_test_helpers::{context, declaration, variable};

#[rstest]
#[case(NameMode::Debug, declaration(None), "app-color-0---255-a-d")]
#[case(
    NameMode::Debug,
    declaration(Some("#ffffff")),
    "app-color-0-_h_FFF--255-a-d"
)]
#[case(NameMode::Debug, variable(), "--app-color-0-")]
#[case(NameMode::Debug, LegacyInput::Keyframes("00123".to_string()), "app-k-00123")]
#[case(NameMode::AtomHoist, declaration(None), "app-a1-h-636f6c6f72-0-n--255")]
#[case(
    NameMode::AtomHoist,
    declaration(Some("")),
    "app-a1-h-636f6c6f72-0-s---255"
)]
#[case(
    NameMode::AtomHoist,
    declaration(Some("red;")),
    "app-a1-h-636f6c6f72-0-s-726564--255"
)]
#[case(NameMode::AtomHoist, variable(), "--app-v1-636f6c6f72-0-")]
#[case(NameMode::AtomHoist, LegacyInput::Keyframes("00123".to_string()), "app-k1-l-61-3030313233")]
#[serial]
fn emits_baseline_golden_when_mode_bypasses_counter(
    #[case] mode: NameMode,
    #[case] input: LegacyInput,
    #[case] expected: &str,
) {
    // Given: an original owner and hoisted delivery with distinct spelling inputs.
    reset_class_map();
    let mut ctx = context(mode, Some(AllocationFile::Original(30)));
    ctx.delivery = Some(CapturedDelivery {
        canonical: "a".to_string(),
        hoisted: true,
    });
    // When: a baseline request is produced.
    let allocated = allocate_name(&input, &ctx);
    // Then: exact baseline bytes and full frozen proof return without a reservation.
    assert_eq!(allocated.name, expected);
    assert_eq!(
        allocated.address,
        NameAddress::Baseline {
            mode,
            input,
            context: ctx,
        }
    );
    assert_eq!(get_class_map(), HashMap::new());
}

#[test]
#[serial]
fn keeps_raw_atom_selector_when_debug_trims_and_encodes_it() {
    // Given: whitespace and punctuation in declaration/variable selectors.
    reset_class_map();
    let inputs = [
        LegacyInput::Declaration(LegacyDeclaration {
            property: " color ".to_string(),
            level: 255,
            value: Some(".8".to_string()),
            selector: Some(" &:hover ".to_string()),
            order: Some(10),
        }),
        LegacyInput::Variable(LegacyVariable {
            property: " color ".to_string(),
            level: 255,
            selector: Some(" &:hover ".to_string()),
        }),
    ];
    let debug = context(NameMode::Debug, None);
    let atom = context(NameMode::AtomHoist, None);
    // When: both baseline modes project the same raw inputs.
    let debug_names = inputs
        .each_ref()
        .map(|input| allocate_name(input, &debug).name);
    let atom_names = inputs
        .each_ref()
        .map(|input| allocate_name(input, &atom).name);
    // Then: 84af trims debug selectors but atom hex preserves their raw bytes.
    assert_eq!(
        debug_names,
        [
            "app-color-255-_d_8-_a__c_hover-10",
            "--app- color -255-_a__c_hover",
        ]
    );
    assert_eq!(
        atom_names,
        [
            "app-a1-g-636f6c6f72-255-s-2e38-20263a686f76657220-10",
            "--app-v1-636f6c6f72-255-20263a686f76657220",
        ]
    );
}

#[test]
#[serial]
fn distinguishes_local_global_and_base_atom_scope_when_delivery_differs() {
    // Given: an unhoisted local canonical bucket and an order0 declaration.
    reset_class_map();
    let mut local = context(NameMode::AtomHoist, Some(AllocationFile::Original(7)));
    local.delivery = Some(CapturedDelivery {
        canonical: "a".to_string(),
        hoisted: false,
    });
    let global = context(NameMode::AtomHoist, None);
    let base = LegacyInput::Declaration(LegacyDeclaration {
        property: "color".to_string(),
        level: 0,
        value: None,
        selector: None,
        order: Some(0),
    });
    // When: atom declarations and keyframes project their established scopes.
    let names = [
        allocate_name(&declaration(None), &local).name,
        allocate_name(&base, &local).name,
        allocate_name(&LegacyInput::Keyframes("spin".to_string()), &global).name,
    ];
    // Then: base declarations are global, while local scope uses captured canonical bytes.
    assert_eq!(
        names,
        [
            "app-a1-l-61-636f6c6f72-0-n--255",
            "app-a1-g-636f6c6f72-0-n--0",
            "app-k1-g-7370696e"
        ]
    );
}

#[test]
#[serial]
fn projects_without_map_mutation_when_fresh_live_state_disagrees() {
    // Given: frozen contexts whose identities do not exist in fresh live maps.
    let ctx = context(
        NameMode::Counter,
        Some(AllocationFile::Legacy {
            filename: "missing.tsx".to_string(),
            ordinal: 30,
        }),
    );
    let atom = context(NameMode::AtomHoist, None);
    let debug = context(NameMode::Debug, None);
    reset_class_map();
    crate::file_map::reset_file_map();
    crate::file_map::reset_canonical_map();
    crate::set_prefix(Some("fresh-".to_string()));
    crate::debug::set_debug(true);
    crate::atom_hoist::set_atom_hoist(Some(3));
    crate::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["fresh.tsx".to_string()])));
    let classes = get_class_map();
    let files = crate::file_map::get_file_map();
    let originals = crate::file_map::get_original_ids();
    let canonical = crate::file_map::get_canonical_map();
    let plan = crate::atom_hoist::atom_plan();
    // When: pure proof functions reconstruct keys and counter/baseline names.
    let key = allocation_key(&declaration(None), &ctx);
    let names = [
        render_name(&declaration(None), &ctx, 30),
        render_name(&declaration(None), &debug, usize::MAX),
        render_name(&declaration(None), &atom, usize::MAX),
    ];
    let baseline_keys = [
        allocation_key(&variable(), &debug),
        allocation_key(&variable(), &atom),
    ];
    // Then: frozen bytes prevail; no map or plan changes and no slot is reserved.
    assert_eq!(
        key,
        Some(("missing.tsx".to_string(), "color-0---255-a-d".to_string()))
    );
    assert_eq!(
        names,
        [
            "app-a-d-a-d",
            "app-color-0---255",
            "app-a1-g-636f6c6f72-0-n--255"
        ]
    );
    assert_eq!(baseline_keys, [None, None]);
    assert_eq!(get_class_map(), classes);
    assert_eq!(crate::file_map::get_file_map(), files);
    assert_eq!(crate::file_map::get_original_ids(), originals);
    assert_eq!(crate::file_map::get_canonical_map(), canonical);
    assert_eq!(crate::atom_hoist::atom_plan(), plan);
    crate::set_prefix(None);
    crate::debug::set_debug(false);
    crate::atom_hoist::set_atom_hoist(None);
    crate::atom_hoist::restore_atom_plan(None);
}

#[test]
#[serial]
fn rolls_back_inner_committed_allocations_when_outer_attempt_drops() {
    // Given: a retained shared declaration outside an attempt.
    reset_class_map();
    let shared = context(NameMode::Counter, None);
    let original = context(NameMode::Counter, Some(AllocationFile::Original(7)));
    let _retained = allocate_name(&declaration(Some("red")), &shared);
    // When: nested extraction commits shared variable/keyframe and private requests, then outer abandons.
    {
        let _outer = Attempt::begin();
        let _outer_name = allocate_name(&declaration(Some("blue")), &shared);
        let inner = Attempt::begin();
        let _variable = allocate_name(&variable(), &shared);
        let _keyframe = allocate_name(&LegacyInput::Keyframes("123".to_string()), &shared);
        let _private = allocate_name(&declaration(None), &original);
        let _reused = allocate_name(&declaration(Some("red")), &shared);
        inner.commit();
    }
    // Then: every new slot rolls back, while reuse preserves its pre-attempt slot.
    assert_eq!(
        get_class_map(),
        HashMap::from([
            (
                String::new(),
                HashMap::from([("color-0-red--255".to_string(), 0)])
            ),
            ("D9-7".to_string(), HashMap::new()),
        ])
    );
    reset_class_map();
}
