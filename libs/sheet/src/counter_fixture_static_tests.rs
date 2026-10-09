use super::counter_fixture_support::{address, fixture, produced, state};
use css::{
    allocation_input::AllocationFile,
    class_map::{get_class_map, set_class_map},
};
use extractor::extract_style::{ProducerPolicy, extract_static_style::ExtractStaticStyle};
use std::collections::HashMap;

#[test]
#[serial_test::serial]
fn static_allocation_reuses_stored_slot_when_constructor_is_cloned_and_deferred() {
    // Given: a genuine constructor and a non-length stored slot plus empty namespace.
    let _state = state();
    let style = fixture("a", || ExtractStaticStyle::new("color", "red", 0, None));
    let before = HashMap::from([
        ("empty".into(), HashMap::new()),
        (
            "D9-0".into(),
            HashMap::from([("color-0-red--255-a".into(), 7)]),
        ),
    ]);
    set_class_map(before.clone());
    let deferred = style.clone();
    // When: production occurs outside TLS under an unrelated later constructor scope.
    let receipt = fixture("later", || {
        produced(deferred.counter_produce(Some("delivery")))
    });
    // Then: the original's actual map slot, not current TLS or namespace length, wins.
    assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(0));
    assert_eq!(receipt.original, 0);
    assert_eq!(receipt.context.file, Some(AllocationFile::Original(0)));
    address(&receipt, ("D9-0", "color-0-red--255-a", 7, "pa-h"));
    assert_eq!(get_class_map(), before);
}

#[test]
#[serial_test::serial]
fn collapsed_delivery_keeps_private_identity_when_originals_have_identical_declarations() {
    // Given: delivery ordinal zero is deliberately unrelated to original ordinals one/two.
    let _state = state();
    fixture("plain", || ());
    let first = fixture("a", || ExtractStaticStyle::new("color", "red", 0, None));
    let second = fixture("b", || ExtractStaticStyle::new("color", "red", 0, None));
    css::file_map::set_canonical_map(HashMap::from([
        ("a".into(), "root".into()),
        ("b".into(), "root".into()),
    ]));
    assert_eq!(css::file_map::get_file_num_by_filename("root"), 0);
    // When: both real deferred adapters allocate into one canonical delivery.
    let a = produced(first.counter_produce(Some("a")));
    let b = produced(second.counter_produce(Some("b")));
    // Then: private IR identity and map namespaces stay distinct from canonical identity.
    assert_ne!(first, second);
    assert_eq!(
        a.context
            .delivery
            .as_ref()
            .map(|delivery| delivery.canonical.as_str()),
        Some("root")
    );
    assert_eq!(
        b.context
            .delivery
            .as_ref()
            .map(|delivery| delivery.canonical.as_str()),
        Some("root")
    );
    address(&a, ("D9-1", "color-0-red--255-b", 0, "pb-a"));
    address(&b, ("D9-2", "color-0-red--255-c", 0, "pc-a"));
    assert_eq!(
        get_class_map(),
        HashMap::from([
            (
                "D9-1".into(),
                HashMap::from([("color-0-red--255-b".into(), 0)])
            ),
            (
                "D9-2".into(),
                HashMap::from([("color-0-red--255-c".into(), 0)])
            ),
        ])
    );
    assert_eq!(
        css::file_map::get_original_ids(),
        std::collections::BTreeMap::from([("plain".into(), 0), ("a".into(), 1), ("b".into(), 2),])
    );
}

#[test]
#[serial_test::serial]
fn basic_order_zero_stays_equal_when_originals_differ_and_delivery_is_private() {
    // Given: new_basic retains genuine authority but intentionally shares order-zero identity.
    let _state = state();
    let a = fixture("a", || {
        ExtractStaticStyle::new_basic("color", "red", 0, None)
    });
    let b = fixture("b", || {
        ExtractStaticStyle::new_basic("color", "red", 0, None)
    });
    // When: adapters receive private filenames after fixture exit.
    let receipts = [
        produced(a.counter_produce(Some("a"))),
        produced(b.counter_produce(Some("b"))),
    ];
    // Then: intentional equality shares the actual slot without erasing policy authority.
    assert_eq!(a, b);
    assert_eq!(a.producer_policy(), ProducerPolicy::CounterOriginal(0));
    assert_eq!(b.producer_policy(), ProducerPolicy::CounterOriginal(1));
    for receipt in receipts {
        address(&receipt, ("", "color-0-red--0", 0, "pa"));
        assert_eq!(receipt.context.file, None);
    }
    assert_eq!(
        get_class_map(),
        HashMap::from([(String::new(), HashMap::from([("color-0-red--0".into(), 0)])),])
    );
}

#[test]
#[serial_test::serial]
fn shared_allocation_reuses_one_slot_when_filename_is_absent() {
    // Given: ordinary static constructors retain different authentic originals.
    let _state = state();
    let a = fixture("a", || ExtractStaticStyle::new("color", "red", 0, None));
    let b = fixture("b", || ExtractStaticStyle::new("color", "red", 0, None));
    // When: both adapters request intentionally shared allocation.
    let receipts = [
        produced(a.counter_produce(None)),
        produced(b.counter_produce(None)),
    ];
    // Then: sharing is explicit, not a fallback to canonical/private numbering.
    for receipt in receipts {
        address(&receipt, ("", "color-0-red--255", 0, "pa"));
        assert_eq!(receipt.context.file, None);
    }
    assert_eq!(
        get_class_map(),
        HashMap::from([(
            String::new(),
            HashMap::from([("color-0-red--255".into(), 0)])
        ),])
    );
}
