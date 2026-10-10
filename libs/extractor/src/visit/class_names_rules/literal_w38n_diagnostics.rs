use super::literal_w38n_inventory::{atom, red_rules};
use super::literal_w38n_source::{TestResult, invalid, observe, offset};
use crate::ErrorDisposition;
use serial_test::serial;

#[test]
#[serial]
fn local_mixin_when_order_is_255_keeps_definitive_diagnostic_and_all_rules() -> TestResult {
    // Given: authored object interpolation and invalid explicit root metadata.
    let source = "tag`background:blue;${{color:'red'}};style-order:255`;";
    // When: the parsed template reaches the uncaptured local literal consumer.
    let actual = observe(source)?;
    // Then: precisely the authored 255 rejects, without losing either declaration.
    assert_eq!(
        actual.errors,
        vec![(offset(source, "255")?, invalid("255"))]
    );
    assert_eq!(actual.disposition, ErrorDisposition::Definitive);
    assert_eq!(actual.inventory, red_rules(None));
    Ok(())
}

#[test]
#[serial]
fn local_mixin_when_order_is_external_keeps_retryable_diagnostic_and_all_rules() -> TestResult {
    // Given: an unresolved authored order hole, not a seeded cache entry.
    let source = "tag`background:blue;${{color:'red'}};style-order:${externalOrder}`;";
    // When: the actual supplier and local consumer process the original source.
    let actual = observe(source)?;
    // Then: only the external order is diagnosed and both rules survive unlayered.
    assert_eq!(
        actual.errors,
        vec![(offset(source, "externalOrder")?, invalid("externalOrder"))]
    );
    assert_eq!(actual.disposition, ErrorDisposition::NeedsEvaluation);
    assert_eq!(actual.inventory, red_rules(None));
    Ok(())
}

#[test]
#[serial]
fn local_mixin_when_call_is_unplaced_keeps_exact_diagnostic_and_ordered_background() -> TestResult {
    // Given: a real call cannot be an UncapturedSource string/template leaf.
    let source = "tag`background:blue;${getRules()};style-order:2`;";
    // When: local literal recognition rejects that authored interpolation.
    let actual = observe(source)?;
    // Then: the call owns one retryable diagnostic; the unrelated rule retains order 2.
    assert_eq!(actual.errors, vec![(offset(source, "getRules()")?,
        "Cannot place `getRules()` at build time: an interpolation in a selector or a property name must be a literal or a constant".to_string())]);
    assert_eq!(actual.disposition, ErrorDisposition::NeedsEvaluation);
    assert_eq!(actual.inventory, vec![atom("background", "blue", Some(2))]);
    Ok(())
}
