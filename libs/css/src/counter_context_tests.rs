use std::collections::{BTreeSet, HashMap};

use rstest::rstest;
use serial_test::serial;

use crate::CounterOwner;
use crate::allocation_input::{
    AllocationFile, CapturedDelivery, LegacyDeclaration, LegacyInput, NameMode, capture_context,
};
use crate::class_map::{get_class_map, reset_class_map};
use crate::counter_names::{allocate_name, allocation_key};
use crate::counter_test_helpers::{declaration, variable};

fn reset() {
    crate::set_prefix(None);
    crate::debug::set_debug(false);
    crate::atom_hoist::set_atom_hoist(None);
    crate::atom_hoist::restore_atom_plan(None);
    crate::file_map::reset_canonical_map();
    crate::file_map::reset_file_map();
    reset_class_map();
}

#[rstest]
#[case(CounterOwner::Inactive)]
#[case(CounterOwner::Unnumbered)]
#[serial]
fn captures_legacy_ordinal_when_low_level_filename_is_supplied(#[case] owner: CounterOwner) {
    // Given: delivery numbering already contains one unrelated file.
    reset();
    let _first = crate::file_map::get_file_num_by_filename("first");
    crate::set_prefix(Some("app-".to_string()));
    // When: the dormant low-level producer captures and allocates two requests for a new filename.
    let first = capture_context(&declaration(None), Some("second"), owner);
    let repeated = capture_context(&declaration(None), Some("second"), owner);
    let allocated = allocate_name(&declaration(None), &first);
    // Then: captured ordinal is reused, and no canonical/original authority is fabricated.
    assert_eq!(first, repeated);
    assert_eq!(
        first.file,
        Some(AllocationFile::Legacy {
            filename: "second".to_string(),
            ordinal: 1
        })
    );
    assert_eq!(allocated.name, "app-b-a");
    assert_eq!(crate::file_map::get_file_map().len(), 2);
    assert_eq!(crate::file_map::get_original_ids().len(), 0);
    reset();
}

#[test]
#[serial]
fn captures_numeric_owner_when_canonical_delivery_has_a_different_number() {
    // Given: two original IDs collapse into delivery root number zero.
    reset();
    crate::file_map::set_canonical_map(HashMap::from([("child".to_string(), "root".to_string())]));
    crate::file_map::seed_file_numbers(&["child".to_string(), "root".to_string()]);
    let before = crate::file_map::get_file_map();
    // When: an explicit original owner is captured for delivery root.
    let ctx = capture_context(&declaration(None), Some("root"), CounterOwner::D9(30));
    let allocated = allocate_name(&declaration(None), &ctx);
    // Then: neither delivery ordinal nor canonical root chooses the private namespace.
    assert_eq!(ctx.file, Some(AllocationFile::Original(30)));
    assert_eq!(allocated.name, "a-d-a");
    assert_eq!(
        allocation_key(&declaration(None), &ctx),
        Some(("D9-30".to_string(), "color-0---255-a-d".to_string()))
    );
    assert_eq!(crate::file_map::get_file_map(), before);
    reset();
}

#[rstest]
#[case(variable(), "--a")]
#[case(LegacyInput::Declaration(LegacyDeclaration {
    property: "color".to_string(), level: 0, value: None, selector: None, order: Some(0),
}), "a")]
#[serial]
fn avoids_file_registration_when_request_is_shared(
    #[case] input: LegacyInput,
    #[case] expected: &str,
) {
    // Given: an unknown delivery filename and explicit private owner.
    reset();
    // When: a shared/base request is captured and allocated.
    let ctx = capture_context(&input, Some("unseen"), CounterOwner::D9(9));
    let allocated = allocate_name(&input, &ctx);
    // Then: no delivery registration or original private namespace occurs.
    assert_eq!(ctx.file, None);
    assert_eq!(ctx.delivery, None);
    assert_eq!(crate::file_map::get_file_map().len(), 0);
    assert_eq!(
        get_class_map().keys().cloned().collect::<Vec<_>>(),
        vec![String::new()]
    );
    assert_eq!(allocated.name, expected);
    reset();
}

#[rstest]
#[case(declaration(Some("red")), true)]
#[case(LegacyInput::Keyframes("123".to_string()), false)]
#[case(variable(), false)]
#[serial]
fn captures_debug_without_unneeded_file_ordinal(
    #[case] input: LegacyInput,
    #[case] registers: bool,
) {
    // Given: debug enabled and a delivery file absent from the ordinal map.
    reset();
    crate::debug::set_debug(true);
    // When: the request captures its baseline mode.
    let ctx = capture_context(&input, Some("a"), CounterOwner::Inactive);
    let allocated = allocate_name(&input, &ctx);
    // Then: only declaration debug requires a file suffix; no counter is reserved.
    assert_eq!(ctx.config.mode, NameMode::Debug);
    assert_eq!(
        crate::file_map::get_file_map().len(),
        usize::from(registers)
    );
    assert_eq!(get_class_map(), HashMap::new());
    assert_eq!(
        allocated.name,
        match input {
            LegacyInput::Declaration(_) => "color-0-red--255-a",
            LegacyInput::Keyframes(_) => "k-123",
            LegacyInput::Variable(_) => "--color-0-",
        }
    );
    reset();
}

#[test]
#[serial]
fn captures_atom_before_debug_when_both_are_enabled() {
    // Given: atom+debug enabled with an explicitly frozen hoisted canonical bucket.
    reset();
    crate::debug::set_debug(true);
    crate::atom_hoist::set_atom_hoist(Some(2));
    crate::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["root".to_string()])));
    crate::file_map::set_canonical_map(HashMap::from([("child".to_string(), "root".to_string())]));
    // When: a declaration captures its original identity and delivery scope.
    let ctx = capture_context(&declaration(None), Some("child"), CounterOwner::D9(7));
    let allocated = allocate_name(&declaration(None), &ctx);
    // Then: atom baseline wins without borrowing/registering a delivery file ordinal.
    assert_eq!(ctx.config.mode, NameMode::AtomHoist);
    assert_eq!(ctx.file, Some(AllocationFile::Original(7)));
    assert_eq!(
        ctx.delivery,
        Some(CapturedDelivery {
            canonical: "root".to_string(),
            hoisted: true
        })
    );
    assert_eq!(allocated.name, "a1-h-636f6c6f72-0-n--255");
    assert_eq!(crate::file_map::get_file_map().len(), 0);
    reset();
}

#[rstest]
#[case(declaration(None), "a1-l-61-636f6c6f72-0-n--255")]
#[case(LegacyInput::Keyframes("123".to_string()), "k1-l-61-313233")]
#[case(variable(), "--v1-636f6c6f72-0-")]
#[serial]
fn avoids_file_ordinals_when_inactive_atom_request_is_captured(
    #[case] input: LegacyInput,
    #[case] expected: &str,
) {
    // Given: atom mode with a frozen empty plan and an unknown legacy filename.
    reset();
    crate::atom_hoist::set_atom_hoist(Some(2));
    crate::atom_hoist::restore_atom_plan(Some(BTreeSet::new()));
    // When: capture and allocation run for each baseline input kind.
    let ctx = capture_context(&input, Some("a"), CounterOwner::Inactive);
    let allocated = allocate_name(&input, &ctx);
    // Then: baseline names need no invented file ordinal or counter reservation.
    assert_eq!(ctx.file, None);
    assert_eq!(allocated.name, expected);
    assert_eq!(crate::file_map::get_file_map().len(), 0);
    assert_eq!(get_class_map(), HashMap::new());
    reset();
}

#[rstest]
#[case(None)]
#[case(Some(String::new()))]
#[serial]
fn captures_empty_prefix_when_prefix_is_absent_or_empty(#[case] prefix: Option<String>) {
    // Given: the two legacy equivalent empty prefix configurations.
    reset();
    crate::set_prefix(prefix);
    // When: a shared counter request captures its config and allocates.
    let ctx = capture_context(&declaration(None), None, CounterOwner::Inactive);
    let allocated = allocate_name(&declaration(None), &ctx);
    // Then: the captured normalized prefix produces the exact unprefixed name.
    assert_eq!(ctx.config.prefix, "");
    assert_eq!(allocated.name, "a");
    reset();
}
