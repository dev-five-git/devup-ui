use super::w38n_mixin_source::{control, fixture, guarded_control, rules};
use super::*;

#[test]
#[serial]
fn public_class_names_mixin_when_order_is_valid_keeps_all_ordered_rules_and_classes() {
    // Given: P2 changes only the invalid sibling's order to valid 2.
    let source = fixture("background:blue;${{color:'red'}};style-order:2");
    control(
        &source,
        rules(&["red"], Some(2)),
        &["background-0-blue--2-a", "color-0-red--2-a"],
    );
}

#[test]
#[serial]
fn public_class_names_mixin_when_order_is_omitted_keeps_all_ordinary_rules_and_classes() {
    // Given: P3 is the same source with no order declaration.
    let source = fixture("background:blue;${{color:'red'}};");
    control(
        &source,
        rules(&["red"], None),
        &["background-0-blue--255-a", "color-0-red--255-a"],
    );
}

#[test]
#[serial]
fn public_class_names_mixin_when_guard_is_true_selects_only_red_and_reads_once() {
    guarded_control(true, &["background-0-blue--2-a", "color-0-red--2-a"]);
}

#[test]
#[serial]
fn public_class_names_mixin_when_guard_is_false_selects_only_green_and_reads_once() {
    guarded_control(false, &["background-0-blue--2-a", "color-0-green--2-a"]);
}
