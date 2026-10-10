use super::compile;
use super::literal_w38m_scalar_observe::{assert_render, observe};
use super::literal_w38m_scalar_oracle::inventory;
use super::literal_w38m_scalar_source::{PRECEDENCE, fixture};
use serial_test::serial;

#[test]
#[serial]
fn scalar_precedence_when_spread_repeats_keeps_effects_and_selects_later_orange() {
    // Given: two actual spread occurrences, an intervening effect and distinct overrides.
    let values = [
        ("opacity", 1),
        ("z-index", 2),
        ("flex-grow", 3),
        ("order", 4),
    ];
    let reference = fixture(PRECEDENCE, false);
    let source = fixture(PRECEDENCE, true);
    // When: public extraction and the original two renders execute each source.
    let omitted = compile(&reference).unwrap_or_else(|error| panic!("{error}"));
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let expected_reference = inventory(&omitted, "orange", &values);
    let expected = inventory(&output, "orange", &values);
    let (reference_renders, reference_trace) = observe(&omitted.code);
    let (renders, trace) = observe(&output.code);
    // Then: the complete nine-rule inventory omits overwritten red/purple and first z-index.
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
        vec![
            "before", "spread", "between", "spread", "after", "built", "get", "get"
        ]
    );
    assert_eq!(
        trace,
        vec![
            "before", "spread", "between", "spread", "after", "built", "get", "callback", "get",
            "callback"
        ]
    );
}
