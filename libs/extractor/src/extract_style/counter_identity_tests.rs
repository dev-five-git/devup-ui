use css::{Naming, Site, sparse_site::SourceFile, style_selector::StyleSelector};
use rstest::rstest;

use super::counter_selector::CounterSelector;
use super::{
    ProducerPolicy,
    extract_dynamic_style::ExtractDynamicStyle,
    extract_keyframes::ExtractKeyframes,
    extract_static_style::ExtractStaticStyle,
    policy_test_support::{assert_laws, set_lengths, trace},
};
use crate::sparse_sites::SiteScope;

fn counter_records(original: u32) -> (ExtractStaticStyle, ExtractDynamicStyle, ExtractKeyframes) {
    let _scope = SiteScope::enter_counter_numbered(original, "tone", &[]);
    (
        ExtractStaticStyle::new_basic("color", "red", 0, None),
        ExtractDynamicStyle::new("color", 0, "tone", None),
        ExtractKeyframes::default(),
    )
}

#[rstest]
#[case(Naming::Own)]
#[case(Naming::Risky)]
fn private_static_originals_survive_unions_when_provenance_changes(#[case] naming: Naming) {
    // Given: equivalent authored declarations constructed under two original owners.
    let (mut first, _, _) = counter_records(4);
    let (mut second, _, _) = counter_records(9);
    first.style_order = Some(255);
    second.style_order = Some(255);
    first.naming = naming;
    second.naming = naming;
    let values = [first.clone(), second.clone(), first.clone()];
    // When: both union implementations discard equal records.
    let lengths = set_lengths(&values);
    // Then: the duplicate dedups but neither private original is lost.
    assert_eq!(lengths, (2, 2));
    assert_eq!(first.producer_policy(), ProducerPolicy::CounterOriginal(4));
    assert_eq!(second.producer_policy(), ProducerPolicy::CounterOriginal(9));
    assert_laws(&values);
}

#[test]
fn shared_static_equivalence_retains_raw_originals_when_order_is_zero() {
    // Given: basic declarations retain distinct original captures before union.
    let (first, _, _) = counter_records(4);
    let (second, _, _) = counter_records(9);
    // When: order0 declarations are compared and unioned.
    let lengths = set_lengths(&[first.clone(), second.clone()]);
    // Then: intentional shared equivalence does not rewrite the stored capture.
    assert_eq!(lengths, (1, 1));
    assert_eq!(trace(&first), trace(&second));
    assert_eq!(first.producer_policy(), ProducerPolicy::CounterOriginal(4));
    assert_eq!(second.producer_policy(), ProducerPolicy::CounterOriginal(9));
}

#[test]
fn private_dynamic_and_empty_frames_survive_unions_when_sites_and_steps_are_absent() {
    // Given: no-site consumers and empty keyframes cannot infer owners from children/sites.
    let (_, first, first_frames) = counter_records(4);
    let (_, second, second_frames) = counter_records(9);
    // When: each kind is unioned after all construction scopes have ended.
    let dynamic_lengths = set_lengths(&[first.clone(), second.clone(), first.clone()]);
    let frame_lengths = set_lengths(&[
        first_frames.clone(),
        second_frames.clone(),
        first_frames.clone(),
    ]);
    // Then: both private original owners survive for each record kind.
    assert_eq!(dynamic_lengths, (2, 2));
    assert_eq!(frame_lengths, (2, 2));
    assert_laws(&[first, second]);
    assert_laws(&[first_frames, second_frames]);
}

#[test]
fn dynamic_sites_remain_distinct_when_order_zero_ignores_allocation_owner() {
    // Given: shared-order consumers still carry different authored variable sites.
    let (_, mut first, _) = counter_records(4);
    let (_, mut second, _) = counter_records(9);
    first.style_order = Some(0);
    second.style_order = Some(0);
    first.site = Some(Site {
        file: SourceFile::D9(4),
        at: 2,
        role: 0,
    });
    second.site = Some(Site {
        file: SourceFile::D9(9),
        at: 2,
        role: 0,
    });
    let mut alternate_role = first.clone();
    alternate_role
        .site
        .as_mut()
        .unwrap_or_else(|| panic!("fixture site"))
        .role = 1;
    // When: shared declarations with distinct variable consumers are unioned.
    let values = [first, second, alternate_role];
    let lengths = set_lengths(&values);
    // Then: site file and syntax role still prevent incompatible consumer merging.
    assert_eq!(lengths, (3, 3));
    assert_laws(&values);
}

#[test]
fn provenance_is_identity_neutral_when_counter_original_is_retained() {
    // Given: static and no-site dynamic copies belong to the same original.
    let (mut style, dynamic, _) = counter_records(4);
    style.style_order = Some(1);
    let mut risky_dynamic = dynamic.clone();
    risky_dynamic.naming = Naming::Risky;
    // When: the copies acquire Risky provenance without construction changes.
    let risky_style = style.clone().with_naming(Naming::Risky);
    // Then: counter equality and hash traces do not reintroduce eligibility.
    assert_eq!(set_lengths(&[style.clone(), risky_style.clone()]), (1, 1));
    assert_eq!(trace(&style), trace(&risky_style));
    assert_eq!(
        set_lengths(&[dynamic.clone(), risky_dynamic.clone()]),
        (1, 1)
    );
    assert_eq!(trace(&dynamic), trace(&risky_dynamic));
}

#[test]
fn no_site_dynamic_records_share_identity_when_order_is_zero() {
    // Given: distinct original captures without authored variable sites.
    let (_, mut first, _) = counter_records(4);
    let (_, mut second, _) = counter_records(9);
    first.style_order = Some(0);
    second.style_order = Some(0);
    // When: intentional shared allocation-owner equivalence is projected.
    let lengths = set_lengths(&[first.clone(), second.clone()]);
    // Then: sharing never erases either retained original capture.
    assert_eq!(lengths, (1, 1));
    assert_eq!(trace(&first), trace(&second));
    assert_eq!(first.producer_policy(), ProducerPolicy::CounterOriginal(4));
    assert_eq!(second.producer_policy(), ProducerPolicy::CounterOriginal(9));
}

#[test]
fn debug_output_stays_unchanged_when_records_carry_counter_policy() {
    // Given: Current and counter records have otherwise identical visible fields.
    let (style, dynamic, frames) = counter_records(4);
    let current = ExtractStaticStyle::new_basic("color", "red", 0, None);
    let current_dynamic = ExtractDynamicStyle::new("color", 0, "tone", None);
    let current_frames = ExtractKeyframes::default();
    // When: the existing custom Debug implementations format both domains.
    let actual = [
        format!("{style:?}"),
        format!("{dynamic:?}"),
        format!("{frames:?}"),
    ];
    // Then: adding policy metadata changes none of the existing debug/snapshot text.
    assert_eq!(
        actual,
        [
            format!("{current:?}"),
            format!("{current_dynamic:?}"),
            format!("{current_frames:?}")
        ]
    );
}

#[test]
fn mixed_domains_obey_total_order_when_shared_and_private_records_coexist() {
    // Given: Current plus shared/private counter records, including duplicate originals.
    let (shared, dynamic, frames) = counter_records(4);
    let (other_shared, other_dynamic, other_frames) = counter_records(9);
    let mut private = shared.clone();
    private.style_order = Some(1);
    let mut private_other = other_shared.clone();
    private_other.style_order = Some(255);
    let current = ExtractStaticStyle::new_basic("color", "red", 0, None);
    let current_dynamic = ExtractDynamicStyle::new("color", 0, "tone", None);
    let current_frames = ExtractKeyframes::default();
    // When: all pairs, triples and both set insertion orders are checked.
    assert_laws(&[
        current.clone(),
        shared.clone(),
        other_shared,
        private,
        private_other,
    ]);
    assert_laws(&[current_dynamic.clone(), dynamic.clone(), other_dynamic]);
    assert_laws(&[current_frames.clone(), frames.clone(), other_frames]);
    // Then: even order0 equivalence never crosses the Current/counter domain boundary.
    assert!(current < shared);
    assert!(current_dynamic < dynamic);
    assert!(current_frames < frames);
}

#[test]
fn counter_selector_order_matches_equality_when_global_cleanup_files_differ() {
    // Given: global records are identical except for cleanup-file semantics.
    let mut statics = vec![];
    let mut dynamics = vec![];
    let mut keyframes = vec![];
    for selector in [
        None,
        Some(StyleSelector::from("&:hover")),
        Some(StyleSelector::from("print")),
        Some(StyleSelector::Global("body".into(), "a.tsx".into())),
        Some(StyleSelector::Global("body".into(), "b.tsx".into())),
    ] {
        let _scope = SiteScope::enter_counter_numbered(7, "tone", &[]);
        let style = ExtractStaticStyle::new("color", "red", 0, selector.clone());
        let mut frames = ExtractKeyframes::default();
        frames.keyframes.insert("from".into(), vec![style.clone()]);
        keyframes.push(frames);
        statics.push(style);
        dynamics.push(ExtractDynamicStyle::new("color", 0, "tone", selector));
    }
    // When: all selector kinds, including the global filename tie, are compared and unioned.
    assert_laws(&statics);
    assert_laws(&dynamics);
    assert_laws(&keyframes);
    for left in &statics {
        for right in &statics {
            let left = CounterSelector(&left.selector);
            let right = CounterSelector(&right.selector);
            assert_eq!(left.partial_cmp(&right), Some(left.cmp(&right)));
        }
    }
    // Then: cleanup-file distinctions survive ordered sets just as they survive hash sets.
    assert_eq!(set_lengths(&statics), (5, 5));
    assert_eq!(set_lengths(&dynamics), (5, 5));
    assert_eq!(set_lengths(&keyframes), (5, 5));
}
