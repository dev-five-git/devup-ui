use super::compile;
use super::literal_w38m_scalar_observe::{Rendered, observe};
use super::literal_w38m_scalar_oracle::inventory;
use super::literal_w38m_scalar_source::throwing;
use serial_test::serial;

#[test]
#[serial]
fn scalar_throw_when_actual_spread_value_throws_preserves_sentinel_and_stops_later_work() {
    // Given: the proven opaque scalar spread source with a real source sentinel catch.
    let values = [("opacity", 1), ("z-index", 2), ("flex-grow", 3)];
    let reference = throwing(false);
    let source = throwing(true);
    // When: construction reaches the original opaque operation in each extracted form.
    let omitted = compile(&reference).unwrap_or_else(|error| panic!("{error}"));
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let reference_inventory = inventory(&omitted, "red", &values);
    let actual_inventory = inventory(&output, "red", &values);
    let reference_observation = observe(&omitted.code);
    let actual = observe(&output.code);
    // Then: emitted rules match input, but no component/getter/callback/later effect runs.
    assert_eq!(actual_inventory.tokens, reference_inventory.tokens);
    assert_eq!(actual_inventory.variables, reference_inventory.variables);
    assert_eq!(actual, reference_observation);
    assert_eq!(
        actual.0,
        vec![
            Rendered {
                class_name: "same".to_string(),
                variables: None
            },
            Rendered {
                class_name: "stopped".to_string(),
                variables: None
            },
        ]
    );
    assert_eq!(actual.1, vec!["before", "opaque"]);
}
