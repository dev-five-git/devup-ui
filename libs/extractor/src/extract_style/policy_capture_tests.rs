use css::{CounterOwner, Naming};
use rstest::rstest;

use super::{
    ProducerPolicy,
    extract_dynamic_style::ExtractDynamicStyle,
    extract_keyframes::ExtractKeyframes,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
    extract_style_value::ExtractStyleValue,
    policy_test_support::trace,
};
use crate::sparse_sites::{SiteScope, producer_policy, retain_folded_owner, site_at, site_errors};

#[test]
fn constructors_capture_original_when_basic_layered_dynamic_and_empty_records_are_built() {
    // Given: only the test counter scope selects the dormant construction input.
    let _scope = SiteScope::enter_counter_numbered(7, "tone", &[]);
    // When: every constructor/default family is used without a variable site or frame member.
    let records = [
        ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None)),
        ExtractStyleValue::Static(ExtractStaticStyle::new_basic("color", "red", 0, None)),
        ExtractStyleValue::Static(ExtractStaticStyle::new_with_layer(
            "color",
            "red",
            0,
            None,
            Some("theme".into()),
        )),
        ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "tone", None)),
        ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
    ];
    // Then: construction, not site/member inference, retains the original in each kind.
    for record in records {
        let policy = match record {
            ExtractStyleValue::Static(style) => style.producer_policy(),
            ExtractStyleValue::Dynamic(style) => {
                assert_eq!(style.counter_owner(), CounterOwner::Inactive);
                style.producer_policy()
            }
            ExtractStyleValue::Keyframes(frames) => {
                assert_eq!(frames.keyframes.len(), 0);
                assert_eq!(frames.origin.0, None);
                assert_eq!(frames.origin.1, None);
                frames.producer_policy()
            }
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_) => panic!("unexpected constructor record"),
        };
        assert_eq!(policy, ProducerPolicy::CounterOriginal(7));
    }
}

#[rstest]
#[case(false)]
#[case(true)]
fn ordinary_scopes_capture_current_when_nested_inside_counter_scope(#[case] numbered: bool) {
    // Given: an outer counter context with deferred records.
    let outer = SiteScope::enter_counter_numbered(7, "outer", &[]);
    let retained = ExtractStaticStyle::new("color", "red", 0, None);
    let before = trace(&retained);
    // When: either ordinary scope path is entered and dropped.
    let current = {
        let _inner = if numbered {
            SiteScope::enter_numbered(9, "inner", &[])
        } else {
            SiteScope::enter("policy-current.tsx", "inner", &[])
        };
        assert_eq!(producer_policy(), ProducerPolicy::Current);
        assert_eq!(trace(&retained), before);
        (
            ExtractStaticStyle::new("color", "red", 0, None),
            ExtractDynamicStyle::new("color", 0, "tone", None),
            ExtractKeyframes::default(),
        )
    };
    drop(outer);
    // Then: existing constructors stay Current, old records stay counter, absent scope is Current.
    assert_eq!(current.0.producer_policy(), ProducerPolicy::Current);
    assert_eq!(current.1.producer_policy(), ProducerPolicy::Current);
    assert_eq!(current.2.producer_policy(), ProducerPolicy::Current);
    assert_eq!(
        retained.producer_policy(),
        ProducerPolicy::CounterOriginal(7)
    );
    assert_eq!(producer_policy(), ProducerPolicy::Current);
    assert_eq!(trace(&retained), before);
}

#[test]
fn nested_counter_scope_restores_sites_and_policy_when_inner_context_ends() {
    // Given: outer folded ownership and a pending source-role error.
    let _outer = SiteScope::enter_counter_numbered(7, "abcdefghijkl", &[]);
    retain_folded_owner(10, 2);
    let _ = site_at(10, 0, "outer");
    let _ = site_at(10, 0, "conflict");
    // When: a nested counter context constructs a record and is dropped.
    let deferred = {
        let _inner = SiteScope::enter_counter_numbered(9, "inner", &[]);
        assert_eq!(site_errors(), vec![]);
        ExtractDynamicStyle::new("color", 0, "tone", None).at(0)
    };
    // Then: outer site/error state restores, while the record retains its inner construction owner.
    assert_eq!(producer_policy(), ProducerPolicy::CounterOriginal(7));
    assert_eq!(site_at(10, 0, "outer").map(|site| site.at), Some(2));
    assert_eq!(site_errors().len(), 1);
    assert_eq!(
        deferred.producer_policy(),
        ProducerPolicy::CounterOriginal(9)
    );
    assert_eq!(deferred.counter_owner(), CounterOwner::D9(9));
}

#[test]
fn site_attachment_preserves_capture_when_other_scope_normalizes_authored_positions() {
    // Given: a consumer is built before receiving a site under another context.
    let style = {
        let _scope = SiteScope::enter_counter_numbered(7, "tone", &[]);
        ExtractDynamicStyle::new("color", 0, "tone", None)
    };
    let edits = [(0, 0, 10)];
    let _other = SiteScope::enter_counter_numbered(9, "\u{feff}a\r\nb", &[&edits]);
    // When: a source-ordered role is attached with BOM/CR and edit normalization.
    let placed = style.at_role(16, 1);
    // Then: variable-site identity is normalized without replacing producer policy.
    let site = placed
        .site()
        .unwrap_or_else(|| panic!("active fixture scope"));
    assert_eq!((site.at, site.role), (2, 1));
    assert_eq!(placed.counter_owner(), CounterOwner::D9(9));
    assert_eq!(placed.producer_policy(), ProducerPolicy::CounterOriginal(7));
}

#[test]
fn record_edits_preserve_capture_when_identifier_order_and_provenance_change() {
    // Given: deferred static and dynamic records have retained one original.
    let (style, mut dynamic) = {
        let _scope = SiteScope::enter_counter_numbered(7, "tone", &[]);
        (
            ExtractStaticStyle::new("color", "$text", 0, None),
            ExtractDynamicStyle::new("color", 0, "tone", None),
        )
    };
    // When: ordinary record edits happen after the scope has ended.
    let style = style
        .with_naming(Naming::Risky)
        .with_theme_token_resolution(ThemeTokenResolution::FirstValue);
    dynamic.replace_identifier("other");
    let mut value = ExtractStyleValue::Dynamic(dynamic);
    value.join_naming(Naming::Risky);
    value.set_style_order(0);
    let ExtractStyleValue::Dynamic(dynamic) = value else {
        panic!("dynamic fixture");
    };
    // Then: no setter, source site or current context rewrites the capture.
    assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(7));
    assert_eq!(
        dynamic.producer_policy(),
        ProducerPolicy::CounterOriginal(7)
    );
    assert_eq!(dynamic.identifier(), "other");
    assert_eq!(dynamic.style_order(), Some(0));
    assert_eq!(dynamic.site(), None);
}

#[test]
fn assignment_site_preserves_capture_when_consumer_is_attached_in_another_scope() {
    // Given: a consumer already belongs to one original before assignment ownership is attached.
    let style = {
        let _scope = SiteScope::enter_counter_numbered(7, "tone", &[]);
        ExtractDynamicStyle::new("color", 0, "tone", None)
    };
    let _scope = SiteScope::enter_numbered(9, "tone", &[]);
    // When: the assignment-site adapter fills the consumer's source location.
    let placed = style.with_assignment_site(0);
    // Then: the variable site follows the assignment but immutable producer authority is retained.
    assert_eq!(placed.counter_owner(), CounterOwner::D9(9));
    assert_eq!(placed.producer_policy(), ProducerPolicy::CounterOriginal(7));
}
