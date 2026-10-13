use super::compile;
use super::literal_w38m_scalar_observe::{assert_render, observe};
use super::literal_w38m_scalar_oracle::inventory;
use super::literal_w38m_scalar_source::{EFFECTS, fixture};
use serial_test::serial;

#[test]
#[serial]
fn scalar_effects_when_nested_bare_order_uses_real_spread_preserve_each_phase_once() {
    // Given: the precise eligible scalar source pair and independent unitless inputs.
    let values = [("opacity", 1), ("z-index", 2), ("flex-grow", 3)];
    let reference = fixture(EFFECTS, false);
    let source = fixture(EFFECTS, true);
    // When: public extraction and the original Oxc/Boa renders execute both forms.
    let omitted = compile(&reference).unwrap_or_else(|error| panic!("{error}"));
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let expected_reference = inventory(&omitted, "red", &values);
    let expected = inventory(&output, "red", &values);
    let (reference_renders, reference_trace) = observe(&omitted.code);
    let (renders, trace) = observe(&output.code);
    // Then: all eight rules select their own tokens/variables and omission matches fully.
    assert_eq!(renders.len(), 2);
    for render in &reference_renders {
        assert_render(render, &expected_reference);
    }
    for render in &renders {
        assert_render(render, &expected);
    }
    assert_eq!(renders, reference_renders);
    assert_eq!(
        reference_trace,
        vec!["before", "spread", "after", "built", "get", "get"]
    );
    assert_eq!(
        trace,
        vec![
            "before", "spread", "after", "built", "get", "callback", "get", "callback"
        ]
    );
}
