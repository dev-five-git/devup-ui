use std::cmp::Ordering;

use rstest::rstest;
use serial_test::serial;

use super::{
    ProducerPolicy,
    extract_dynamic_style::ExtractDynamicStyle,
    numeric_conversion::{NumericConversion, NumericUnit},
    policy_test_support::{assert_laws, set_lengths, trace},
};
use crate::sparse_sites::SiteScope;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn lowering_metadata_survives_unions_when_dynamic_records_share_every_other_field(
    #[case] counter: bool,
) {
    // Given: the same source record can carry distinct lowering semantics.
    let _scope = counter.then(|| SiteScope::enter_counter_numbered(4, "value", &[]));
    let plain = ExtractDynamicStyle::new("padding", 0, "value", None);
    let scaled = plain
        .clone()
        .with_conversion(NumericConversion::Number(NumericUnit::Length));
    let present = plain.clone().with_presence();
    let values = [
        plain.clone(),
        scaled.clone(),
        present.clone(),
        plain.clone(),
    ];
    // When: production hash and ordered collection identities remove duplicates.
    let lengths = set_lengths(&values);
    // Then: only the identical copy deduplicates, never a different declaration/class policy.
    assert_eq!(lengths, (3, 3));
    assert_ne!(plain, scaled);
    assert_ne!(plain, present);
    assert_ne!(trace(&plain), trace(&scaled));
    assert_ne!(trace(&plain), trace(&present));
    assert_ne!(plain.cmp(&scaled), Ordering::Equal);
    assert_ne!(plain.cmp(&present), Ordering::Equal);
    assert_eq!(trace(&plain), trace(&values[3]));
    assert_laws(&values);
    assert_eq!(
        plain.effective_value(),
        format!("var({})", plain.variable_name())
    );
    assert_eq!(
        scaled.effective_value(),
        format!("calc(var({}) * 4px)", scaled.variable_name())
    );
    assert_eq!(present.effective_value(), plain.effective_value());
    assert!(!plain.presence());
    assert!(present.presence());
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn all_lowering_variants_keep_distinct_identity_when_duplicates_are_unioned(#[case] counter: bool) {
    // Given: every conversion/unit and presence combination has the same source fields.
    let _scope = counter.then(|| SiteScope::enter_counter_numbered(4, "value", &[]));
    let base = ExtractDynamicStyle::new("padding", 0, "value", None);
    let mut values = vec![];
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
        values.push(converted.clone());
        values.push(converted.with_presence());
    }
    values.extend(values.clone());
    // When: both collection implementations remove exact copies.
    let lengths = set_lengths(&values);
    // Then: all 14 lowering identities survive, and copies obey Eq/Hash/Ord laws.
    assert_eq!(lengths, (14, 14));
    assert_laws(&values);
    for left in &values {
        assert_eq!(
            left.producer_policy(),
            if counter {
                ProducerPolicy::CounterOriginal(4)
            } else {
                ProducerPolicy::Current
            }
        );
        for right in &values {
            assert_eq!(
                left.cmp(right),
                left.conversion()
                    .cmp(&right.conversion())
                    .then(left.presence().cmp(&right.presence()))
            );
            if left != right {
                assert_ne!(trace(left), trace(right));
            }
        }
    }
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn earlier_fields_keep_order_when_lowering_metadata_orders_the_other_way(#[case] counter: bool) {
    // Given: a smaller original order has larger conversion and presence metadata.
    let _scope = counter.then(|| SiteScope::enter_counter_numbered(4, "value", &[]));
    let mut first = ExtractDynamicStyle::new("padding", 0, "value", None)
        .with_conversion(NumericConversion::Unknown(NumericUnit::Time))
        .with_presence();
    first.style_order = Some(0);
    let mut second = ExtractDynamicStyle::new("padding", 0, "value", None);
    second.style_order = Some(1);
    // When: the records are ordered by the production projection.
    let order = first.cmp(&second);
    // Then: lowering metadata never moves ahead of the preexisting order field.
    assert_eq!(order, Ordering::Less);
}
