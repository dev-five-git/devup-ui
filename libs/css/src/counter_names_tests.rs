use std::collections::HashMap;

use rstest::rstest;
use serial_test::serial;

use crate::allocation_input::{
    AllocationFile, LegacyDeclaration, LegacyInput, LegacyVariable, NameMode,
};
use crate::class_map::{get_class_map, reset_class_map, set_class_map};
use crate::counter_names::{NameAddress, allocate_name, allocation_key};
use crate::counter_test_helpers::{context, declaration, variable};

#[test]
#[serial]
fn preserves_ordered_751_trace_when_streams_interleave() {
    // Given: shared and legacy private contexts with independent streams.
    reset_class_map();
    let shared = context(NameMode::Counter, None);
    let private = context(
        NameMode::Counter,
        Some(AllocationFile::Legacy {
            filename: "b.tsx".to_string(),
            ordinal: 1,
        }),
    );
    let base = LegacyInput::Declaration(LegacyDeclaration {
        property: "color".to_string(),
        level: 0,
        value: Some("red".to_string()),
        selector: None,
        order: Some(0),
    });
    let animation = LegacyInput::Keyframes("18446744073709551615".to_string());
    // When: requests alternate shared/private declarations, keyframes and variables.
    let names = [
        allocate_name(&declaration(Some("red")), &shared),
        allocate_name(&declaration(Some("red")), &private),
        allocate_name(&animation, &shared),
        allocate_name(&variable(), &private),
        allocate_name(&animation, &private),
        allocate_name(&base, &private),
        allocate_name(&declaration(Some("blue")), &private),
        allocate_name(&declaration(Some("red")), &shared),
    ];
    // Then: exact 84af request arithmetic yields pinned names and map addresses.
    assert_eq!(
        names.map(|allocated| allocated.name),
        [
            "app-a", "app-b-a", "app-b", "--app-c", "app-b-b", "app-d", "app-b-c", "app-a",
        ]
    );
    assert_eq!(
        get_class_map(),
        HashMap::from([
            (
                String::new(),
                HashMap::from([
                    ("color-0-red--255".to_string(), 0),
                    ("k-18446744073709551615".to_string(), 1),
                    ("color-0-".to_string(), 2),
                    ("color-0-red--0".to_string(), 3),
                ])
            ),
            (
                "b.tsx".to_string(),
                HashMap::from([
                    ("color-0-red--255-b".to_string(), 0),
                    ("k-18446744073709551615".to_string(), 1),
                    ("color-0-blue--255-b".to_string(), 2),
                ])
            ),
        ])
    );
    reset_class_map();
}

#[rstest]
#[case(None, "color-0---255")]
#[case(Some(""), "color-0---255")]
#[case(Some("  red;  "), "color-0-red--255")]
#[case(Some("#ffffff"), "color-0-#FFF--255")]
#[case(Some("rgba(255, 0, 0,    0.5)"), "color-0-#FF000080--255")]
#[case(Some("0px"), "color-0-0--255")]
#[serial]
fn reuses_normalized_key_when_raw_input_differs(#[case] value: Option<&str>, #[case] key: &str) {
    // Given: the golden normalized key already owns slot 30, not map length 1.
    set_class_map(HashMap::from([(
        String::new(),
        HashMap::from([(key.to_string(), 30)]),
    )]));
    let ctx = context(NameMode::Counter, None);
    // When: the raw input requests its legacy allocation.
    let allocated = allocate_name(&declaration(value), &ctx);
    // Then: the actual stored slot, including ad splice, is returned without another key.
    assert_eq!(allocated.name, "app-a-d");
    assert_eq!(
        allocated.address,
        NameAddress::Counter {
            namespace: String::new(),
            legacy_key: key.to_string(),
            slot: 30,
        }
    );
    assert_eq!(get_class_map()[""].len(), 1);
    reset_class_map();
}

#[test]
#[serial]
fn preserves_selector_order_and_raw_variable_property_when_keys_are_built() {
    // Given: normalized declaration dimensions and a variable with significant property whitespace.
    reset_class_map();
    let ctx = context(NameMode::Counter, None);
    let inputs = [
        LegacyInput::Declaration(LegacyDeclaration {
            property: " color ".to_string(),
            level: 10,
            value: Some(" red; ".to_string()),
            selector: Some(" &:hover ".to_string()),
            order: Some(100),
        }),
        LegacyInput::Variable(LegacyVariable {
            property: " color ".to_string(),
            level: 10,
            selector: Some(" &:hover ".to_string()),
        }),
        LegacyInput::Variable(LegacyVariable {
            property: "color".to_string(),
            level: 10,
            selector: Some("&:hover".to_string()),
        }),
    ];
    // When: each distinct legacy request is allocated in sequence.
    let names = inputs
        .each_ref()
        .map(|input| allocate_name(input, &ctx).name);
    // Then: only declaration property and selector whitespace normalize.
    assert_eq!(names, ["app-a", "--app-b", "--app-c"]);
    assert_eq!(
        get_class_map()[""],
        HashMap::from([
            ("color-10-red-&:hover-100".to_string(), 0),
            (" color -10-&:hover".to_string(), 1),
            ("color-10-&:hover".to_string(), 2),
        ])
    );
    reset_class_map();
}

#[rstest]
#[case(26, "_")]
#[case(27, "aa")]
#[case(30, "a-d")]
#[case(36, "aj")]
#[serial]
fn retains_file_and_slot_encoding_when_boundaries_are_crossed(
    #[case] ordinal: usize,
    #[case] encoded: &str,
) {
    // Given: a dense private map and a captured file ordinal at the same boundary.
    let ctx = context(
        NameMode::Counter,
        Some(AllocationFile::Legacy {
            filename: "boundary".to_string(),
            ordinal,
        }),
    );
    set_class_map(HashMap::from([(
        "boundary".to_string(),
        (0..ordinal)
            .map(|index| (format!("seed-{index}"), index))
            .collect(),
    )]));
    // When: a new declaration receives the next actual slot.
    let allocated = allocate_name(&declaration(Some("red")), &ctx);
    // Then: prefix/file/slot splices remain independent, and address is numeric.
    assert_eq!(allocated.name, format!("app-{encoded}-{encoded}"));
    assert_eq!(
        allocated.address,
        NameAddress::Counter {
            namespace: "boundary".to_string(),
            legacy_key: format!("color-0-red--255-{encoded}"),
            slot: ordinal,
        }
    );
    reset_class_map();
}

#[test]
#[serial]
fn isolates_original_owner_when_delivery_ordinal_is_supplied() {
    // Given: original 30 and delivery ordinal 1, never an owner fallback.
    reset_class_map();
    let original = context(NameMode::Counter, Some(AllocationFile::Original(30)));
    let delivery = context(
        NameMode::Counter,
        Some(AllocationFile::Legacy {
            filename: "root.tsx".to_string(),
            ordinal: 1,
        }),
    );
    // When: the same declaration is requested in both allocation domains.
    let names = [
        allocate_name(&declaration(None), &original),
        allocate_name(&declaration(None), &delivery),
    ];
    // Then: original numeric authority is separate from the supplied delivery stream.
    assert_eq!(names[0].name, "app-a-d-a");
    assert_eq!(names[1].name, "app-b-a");
    assert_eq!(
        allocation_key(&declaration(None), &original),
        Some(("D9-30".to_string(), "color-0---255-a-d".to_string(),))
    );
    assert_eq!(get_class_map().len(), 2);
    reset_class_map();
}

#[test]
#[serial]
fn reuses_none_empty_and_default_order_when_legacy_inputs_are_equivalent() {
    // Given: a shared declaration stream and raw requests with equivalent legacy keys.
    reset_class_map();
    let ctx = context(NameMode::Counter, None);
    let mut explicit = LegacyDeclaration {
        property: "color".to_string(),
        level: 0,
        value: Some(String::new()),
        selector: Some("  ".to_string()),
        order: Some(255),
    };
    let inputs = [
        declaration(None),
        LegacyInput::Declaration(explicit.clone()),
        LegacyInput::Declaration({
            explicit.order = Some(1);
            explicit.clone()
        }),
        LegacyInput::Declaration({
            explicit.selector = Some("hover".to_string());
            explicit
        }),
    ];
    // When: equivalent and genuinely distinct requests run in authored order.
    let names = inputs
        .each_ref()
        .map(|input| allocate_name(input, &ctx).name);
    // Then: None/empty/default reuse one slot, while actual order/selector remain key dimensions.
    assert_eq!(names, ["app-a", "app-a", "app-b", "app-c"]);
    assert_eq!(
        get_class_map()[""],
        HashMap::from([
            ("color-0---255".to_string(), 0),
            ("color-0---1".to_string(), 1),
            ("color-0--hover-1".to_string(), 2),
        ])
    );
    reset_class_map();
}
