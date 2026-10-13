use super::literal_w38n_inventory::{Inventory, atom};
use super::literal_w38n_selection::selected;
use super::literal_w38n_source::{TestResult, observe};
use crate::ErrorDisposition;
use serial_test::serial;

fn conditional_selection(flag: bool, expected: &[&str]) -> TestResult {
    // Given: real conditional object alternatives, with no injected LocalKnownPart.
    let source = "tag`background:blue;${flag?{color:'red'}:{color:'green'}};style-order:2`;";
    // When: the authored branches reach literal_scope_local and its real emitter.
    let actual = observe(source)?;
    // Then: the whole carrier retains both arms, while this render selects only one.
    assert_eq!(actual.errors, vec![]);
    assert_eq!(actual.disposition, ErrorDisposition::NeedsEvaluation);
    assert_eq!(
        actual.inventory,
        vec![
            atom("background", "blue", Some(2)),
            Inventory::Conditional {
                test: "!flag".to_string(),
                yes: Some(Box::new(atom("color", "green", Some(2)))),
                no: Some(Box::new(Inventory::Conditional {
                    test: "flag".to_string(),
                    yes: Some(Box::new(atom("color", "red", Some(2)))),
                    no: None,
                })),
            },
        ]
    );
    assert_eq!(selected(&actual.classes, flag)?, expected);
    Ok(())
}

#[test]
#[serial]
fn local_mixin_when_condition_is_true_selects_only_red() -> TestResult {
    conditional_selection(true, &["background-0-blue--2", "color-0-red--2"])
}

#[test]
#[serial]
fn local_mixin_when_condition_is_false_selects_only_green() -> TestResult {
    conditional_selection(false, &["background-0-blue--2", "color-0-green--2"])
}
