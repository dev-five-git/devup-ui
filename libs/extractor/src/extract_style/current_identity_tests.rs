use css::{
    CounterOwner, Naming, Site,
    sparse_site::SourceFile,
    style_origin::{Origin, StyleOrigin},
    style_selector::StyleSelector,
};
use rstest::rstest;

use super::{
    ProducerPolicy,
    extract_dynamic_style::ExtractDynamicStyle,
    extract_keyframes::ExtractKeyframes,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
    numeric_conversion::{NumericConversion, NumericUnit},
    policy_test_support::{old_dynamic, old_keyframes, old_static, set_lengths, trace},
};

fn static_fixtures() -> Vec<ExtractStaticStyle> {
    let mut styles = vec![];
    for naming in [Naming::Own, Naming::Risky] {
        for owner in [
            CounterOwner::Inactive,
            CounterOwner::Unnumbered,
            CounterOwner::D9(0),
            CounterOwner::D9(9),
        ] {
            for order in [None, Some(0), Some(1), Some(255)] {
                for resolution in [
                    ThemeTokenResolution::CssVariable,
                    ThemeTokenResolution::FirstValue,
                ] {
                    let mut style = ExtractStaticStyle::new_with_layer(
                        "color",
                        "$text",
                        2,
                        Some(StyleSelector::from("&:hover")),
                        Some("theme".into()),
                    );
                    style.naming = naming;
                    style.counter_owner = owner;
                    style.style_order = order;
                    style.theme_token_resolution = resolution;
                    styles.push(style);
                }
            }
        }
    }
    styles.push(ExtractStaticStyle::new("background", "red", 0, None));
    for file in ["a.tsx", "b.tsx"] {
        styles.push(ExtractStaticStyle::new(
            "color",
            "red",
            0,
            Some(StyleSelector::Global("body".into(), file.into())),
        ));
    }
    styles
}

#[test]
fn static_hash_writes_match_old_projection_when_policy_is_current() {
    // Given: owner eligibility, order and resolution boundaries in the old domain.
    let styles = static_fixtures();
    // When: records and independent old-field projections are hashed.
    let traces: Vec<_> = styles
        .iter()
        .map(|style| (trace(style), trace(&old_static(style))))
        .collect();
    // Then: no policy byte or changed field sequence reaches the hasher.
    for (actual, expected) in traces {
        assert_eq!(actual, expected);
    }
}

#[test]
fn static_sets_and_order_match_old_projection_when_policy_is_current() {
    // Given: both meaningful and deliberately equivalent owner dimensions.
    let styles = static_fixtures();
    let old: Vec<_> = styles.iter().map(old_static).collect();
    // When: both domains are compared and unioned.
    let lengths = set_lengths(&styles);
    // Then: equality, total ordering and set cardinalities remain the old contract.
    assert_eq!(
        lengths,
        (
            old.iter().collect::<std::collections::HashSet<_>>().len(),
            old.iter().collect::<std::collections::BTreeSet<_>>().len()
        )
    );
    for (left, old_left) in styles.iter().zip(&old) {
        for (right, old_right) in styles.iter().zip(&old) {
            assert_eq!(left == right, old_left == old_right);
            assert_eq!(left.cmp(right), old_left.cmp(old_right));
            assert_eq!(left.partial_cmp(right), old_left.partial_cmp(old_right));
        }
    }
}

fn dynamic_fixtures() -> Vec<ExtractDynamicStyle> {
    let mut styles = vec![];
    for naming in [Naming::Own, Naming::Risky] {
        for order in [None, Some(0), Some(1), Some(255)] {
            for file in [
                None,
                Some(SourceFile::D9(0)),
                Some(SourceFile::Unnumbered("received".into())),
            ] {
                for important in [false, true] {
                    let mut style = ExtractDynamicStyle::new(
                        "color",
                        2,
                        if important { "tone !important" } else { "tone" },
                        Some(StyleSelector::from("&:hover")),
                    );
                    style.naming = naming;
                    style.style_order = order;
                    style.layer = Some("theme".into());
                    style.site = file.clone().map(|file| Site {
                        file,
                        at: 8,
                        role: 1,
                    });
                    styles.push(style);
                }
            }
        }
    }
    styles.push(ExtractDynamicStyle::new("opacity", 0, "other", None));
    for file in ["a.tsx", "b.tsx"] {
        styles.push(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(StyleSelector::Global("body".into(), file.into())),
        ));
    }
    let base = styles[0].clone();
    for conversion in [
        NumericConversion::Keep,
        NumericConversion::Number(NumericUnit::Length),
        NumericConversion::Number(NumericUnit::Time),
        NumericConversion::String(NumericUnit::Length),
        NumericConversion::String(NumericUnit::Time),
        NumericConversion::Unknown(NumericUnit::Length),
        NumericConversion::Unknown(NumericUnit::Time),
    ] {
        let converted = base.clone().with_conversion(conversion);
        styles.push(converted.clone());
        styles.push(converted.with_presence());
    }
    styles
}

#[test]
fn dynamic_identity_matches_old_projection_when_policy_is_current() {
    // Given: site/no-site, important, provenance and actual-order boundaries.
    let styles = dynamic_fixtures();
    // When: the new records are projected into the old derived-field sequence.
    let old: Vec<_> = styles.iter().map(old_dynamic).collect();
    // Then: the exact hash writes, equality and ordering match, including optional sites.
    for (left, old_left) in styles.iter().zip(&old) {
        assert_eq!(left.producer_policy(), ProducerPolicy::Current);
        assert_eq!(trace(left), trace(old_left));
        for (right, old_right) in styles.iter().zip(&old) {
            assert_eq!(left == right, old_left == old_right);
            assert_eq!(left.cmp(right), old_left.cmp(old_right));
        }
    }
    assert_eq!(
        set_lengths(&styles),
        (
            old.iter().collect::<std::collections::HashSet<_>>().len(),
            old.iter().collect::<std::collections::BTreeSet<_>>().len()
        )
    );
}

#[rstest]
#[case(false)]
#[case(true)]
fn keyframe_identity_matches_old_projection_when_steps_are_present_or_empty(
    #[case] populated: bool,
) {
    // Given: ordered steps with Current children and a distinct diagnostic origin.
    let mut first = ExtractKeyframes::default();
    if populated {
        first.keyframes.insert(
            "to".into(),
            vec![ExtractStaticStyle::new("opacity", "1", 0, None)],
        );
        first.keyframes.insert(
            "from".into(),
            vec![ExtractStaticStyle::new_basic("opacity", "0", 0, None)],
        );
    }
    let mut second = first.clone();
    second.origin = Origin(
        Some(Box::new(StyleOrigin {
            file: "elsewhere.tsx".into(),
            line: 4,
            column: 9,
            expression: "frames()".into(),
        })),
        None,
    );
    let styles = [ExtractKeyframes::default(), first, second];
    // When: frames and independent recursively old child projections are hashed/compared.
    let old: Vec<_> = styles.iter().map(old_keyframes).collect();
    // Then: empty defaults, ordered maps and diagnostic-neutral identity are unchanged.
    for (left, old_left) in styles.iter().zip(&old) {
        assert_eq!(trace(left), trace(old_left));
        for (right, old_right) in styles.iter().zip(&old) {
            assert_eq!(left == right, old_left == old_right);
            assert_eq!(left.cmp(right), old_left.cmp(old_right));
        }
    }
    assert_eq!(
        set_lengths(&styles),
        (
            old.iter().collect::<std::collections::HashSet<_>>().len(),
            old.iter().collect::<std::collections::BTreeSet<_>>().len()
        )
    );
}
