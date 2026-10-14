use super::counter_producer_test_support::{
    address, cleanup, produced, reset, scope, static_style,
};
use super::{ExtractDynamicStyle, ExtractKeyframes};
use css::{
    allocation_input::NameMode,
    class_map::{Attempt, get_class_map},
    counter_names::{allocation_key, render_name},
};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(None, "pa", "--pc", "pd")]
#[case(Some("delivery"), "pa-a", "--pa", "pa-c")]
#[serial]
fn shared_stream_interleaves_when_static_dynamic_and_keyframes_request_names(
    #[case] filename: Option<&str>,
    #[case] first: &str,
    #[case] variable: &str,
    #[case] frame: &str,
) {
    // Given: one captured original with no-site dynamic and empty frames.
    reset(NameMode::Counter);
    let _scope = scope("original");
    let static_record = static_style("original");
    let dynamic = ExtractDynamicStyle::new("width", 0, "tone", None);
    let frames = ExtractKeyframes::default();
    // When: established traversal requests static, dynamic, then keyframes.
    let s = produced(static_record.counter_produce(filename));
    let d = produced(dynamic.counter_produce(filename));
    let k = produced(frames.counter_produce(filename));
    // Then: CLASS precedes VARIABLE; keyframes compete in the declaration stream.
    assert_eq!(s.allocation.name, first);
    assert_eq!(
        d.class.allocation.name,
        if filename.is_some() { "pa-b" } else { "pb" }
    );
    assert_eq!(d.variable, variable);
    assert_eq!(k.allocation.name, frame);
    address(
        &d.class,
        if filename.is_some() { "D9-0" } else { "" },
        if filename.is_some() {
            "width-0---255-a"
        } else {
            "width-0---255"
        },
        1,
    );
    let v = d
        .variable_allocation
        .as_ref()
        .unwrap_or_else(|| panic!("variable receipt"));
    address(v, "", "width-0-", if filename.is_some() { 0 } else { 2 });
    let before = get_class_map();
    assert_eq!(produced(dynamic.counter_produce(filename)), d);
    assert_eq!(produced(frames.counter_produce(filename)), k);
    assert_eq!(get_class_map(), before);
    cleanup();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn caller_rollback_owns_both_requests_when_inner_attempt_commits(#[case] private: bool) {
    // Given: a preseeded static slot must survive an abandoned caller attempt.
    reset(NameMode::Counter);
    let _scope = scope("original");
    let filename = private.then_some("delivery");
    let seed = produced(static_style("original").counter_produce(filename));
    let dynamic = ExtractDynamicStyle::new("width", 0, "tone", None);
    // When: an inner committed adapter use is followed by outer abandonment.
    let abandoned = {
        let _outer = Attempt::begin();
        assert_eq!(
            produced(static_style("original").counter_produce(filename)),
            seed
        );
        let inner = Attempt::begin();
        let receipt = produced(dynamic.counter_produce(filename));
        inner.commit();
        receipt
    };
    assert_eq!(
        get_class_map()
            .values()
            .map(std::collections::HashMap::len)
            .sum::<usize>(),
        1
    );
    let retried = produced(dynamic.counter_produce(filename));
    // Then: both new requests retry at their old slots, while preseed reuse survives.
    assert_eq!(retried, abandoned);
    assert_eq!(
        produced(static_style("original").counter_produce(filename)),
        seed
    );
    assert_eq!(
        get_class_map()
            .values()
            .map(std::collections::HashMap::len)
            .sum::<usize>(),
        3
    );
    cleanup();
}

#[test]
#[serial]
fn reused_dynamic_survives_when_later_attempt_is_abandoned() {
    // Given: both requests already exist before a caller begins.
    reset(NameMode::Counter);
    let _scope = scope("original");
    let dynamic = ExtractDynamicStyle::new("color", 0, "tone", None);
    let seed = produced(dynamic.counter_produce(Some("delivery")));
    let before = get_class_map();
    // When: the caller reuses both reservations and drops its attempt.
    {
        let _attempt = Attempt::begin();
        assert_eq!(produced(dynamic.counter_produce(Some("delivery"))), seed);
    }
    // Then: reused class and variable evidence remain valid.
    assert_eq!(get_class_map(), before);
    cleanup();
}

#[test]
#[serial]
fn ad_boundary_is_preserved_when_original_and_slot_both_need_splices() {
    // Given: original 30 and thirty independent declarations preseed the private stream.
    reset(NameMode::Counter);
    css::file_map::set_original_ids((0..31).map(|id| (format!("f{id:02}"), id)).collect());
    let _scope = scope("f30");
    for number in 0..30 {
        let style = super::extract_static_style::ExtractStaticStyle::new(
            "--x",
            &number.to_string(),
            0,
            None,
        );
        produced(style.counter_produce(Some("delivery")));
    }
    // When: slot 30 is actually allocated.
    let receipt = produced(static_style("f30").counter_produce(Some("delivery")));
    // Then: both file and slot use the baseline ad-blocker splice.
    assert_eq!(receipt.allocation.name, "pa-d-a-d");
    address(&receipt, "D9-30", "color-0-red--255-a-d", 30);
    cleanup();
}

#[test]
#[serial]
fn pure_receipt_projections_leave_maps_unchanged_when_live_configuration_changes() {
    // Given: one real allocation captured with prefix p and private placement.
    reset(NameMode::Counter);
    let style = static_style("original");
    let receipt = produced(style.counter_produce(Some("delivery")));
    let before = get_class_map();
    css::set_prefix(Some("changed".into()));
    css::debug::set_debug(true);
    css::atom_hoist::set_atom_hoist(Some(2));
    // When: captured proof operations and cloned equality are evaluated.
    let key = allocation_key(&receipt.input, &receipt.context);
    let name = render_name(&receipt.input, &receipt.context, 0);
    assert_eq!(receipt, receipt.clone());
    // Then: pure projection uses frozen authority and performs no new reservation.
    assert_eq!(key, Some(("D9-0".into(), "color-0-red--255-a".into())));
    assert_eq!(name, "pa-a");
    assert_eq!(get_class_map(), before);
    cleanup();
}
