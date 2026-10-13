use super::literal_w38n_inventory::red_rules;
use super::literal_w38n_selection::selected;
use super::literal_w38n_source::{TestResult, observe};
use crate::ErrorDisposition;
use serial_test::serial;

#[test]
#[serial]
fn local_mixin_when_order_is_valid_keeps_all_rules_and_selected_classes() -> TestResult {
    // Given: the valid sibling differs only in explicit order.
    let source = "tag`background:blue;${{color:'red'}};style-order:2`;";
    // When: the actual local consumer emits its class selection.
    let actual = observe(source)?;
    // Then: no error, complete order-2 inventory and precisely both classes.
    assert_eq!(actual.errors, vec![]);
    assert_eq!(actual.disposition, ErrorDisposition::NeedsEvaluation);
    assert_eq!(actual.inventory, red_rules(Some(2)));
    assert_eq!(
        selected(&actual.classes, true)?,
        vec!["background-0-blue--2", "color-0-red--2"]
    );
    Ok(())
}

#[test]
#[serial]
fn local_mixin_when_order_is_omitted_keeps_all_unlayered_rules_and_classes() -> TestResult {
    // Given: the same object interpolation with no metadata.
    let source = "tag`background:blue;${{color:'red'}};`;";
    // When: the source supplier feeds the orderless local consumer.
    let actual = observe(source)?;
    // Then: no metadata is synthesized and both ordinary classes remain.
    assert_eq!(actual.errors, vec![]);
    assert_eq!(actual.disposition, ErrorDisposition::NeedsEvaluation);
    assert_eq!(actual.inventory, red_rules(None));
    assert_eq!(
        selected(&actual.classes, false)?,
        vec!["background-0-blue--255", "color-0-red--255"]
    );
    Ok(())
}
