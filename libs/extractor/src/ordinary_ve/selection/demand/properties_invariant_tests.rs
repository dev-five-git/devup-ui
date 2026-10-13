use oxc_span::Span;
use oxc_syntax::node::NodeId;

use super::super::receiver_test_support::{TestResult, observe, path, span};
use super::super::{Demand, View};

fn owner(view: &View, name: &str) -> TestResult<NodeId> {
    view.selection
        .units
        .iter()
        .find(|unit| unit.bindings.iter().any(|binding| binding.name == name))
        .map(|unit| unit.node)
        .ok_or_else(|| "missing real binding owner".into())
}

fn assert_rendered(code: &str, retained: &[&str], omitted: &[&str]) -> TestResult {
    let mut remaining = code;
    for &text in retained {
        let (_, rest) = remaining
            .split_once(text)
            .ok_or("missing or out-of-order retained property")?;
        remaining = rest;
    }
    for &text in omitted {
        assert!(!code.contains(text), "{code}");
    }
    Ok(())
}

#[test]
fn pruning_declines_when_a_computed_initializer_key_is_genuinely_unknown() -> TestResult {
    // Given
    let source = "export const tokens={space:'8px',[window.name]:'16px',browser:window.document};";
    let demand = path(&["tokens", "space"]);
    // When
    let observed = observe("/unknown-key.ts", source, &demand)?;
    // Then
    let tokens = owner(&observed.view, "tokens")?;
    assert_eq!(observed.view.selection.units.len(), 1);
    assert_eq!(observed.view.units.get(&tokens), Some(&path(&["space"])));
    assert!(!observed.view.properties.contains_key(&tokens));
    assert_eq!(observed.view.properties.len(), 0);
    assert_rendered(
        &observed.rendered.mapped.code,
        &[
            "space:'8px'",
            "[window.name]:'16px'",
            "browser:window.document",
        ],
        &[],
    )?;
    Ok(())
}

#[test]
fn demanded_duplicates_keep_both_original_entries_when_names_are_equal() -> TestResult {
    // Given
    let source = "export const tokens={z:'first',space:'4px',a:'middle',['space']:'8px',browser:window.document};";
    let retained = ["space:'4px'", "['space']:'8px'"];
    let omitted = ["z:'first'", "a:'middle'", "browser:window.document"];
    let demand = path(&["tokens", "space"]);
    // When
    let observed = observe("/ordered-duplicates.ts", source, &demand)?;
    // Then
    let tokens = owner(&observed.view, "tokens")?;
    assert_eq!(observed.view.units.get(&tokens), Some(&path(&["space"])));
    assert_eq!(observed.view.properties.len(), 1);
    let properties = observed
        .view
        .properties
        .get(&tokens)
        .ok_or("missing tokens pruning plan")?;
    assert_eq!(
        properties
            .kept
            .iter()
            .map(|(span, nested)| (*span, nested.is_none()))
            .collect::<Vec<_>>(),
        vec![
            (span(source, retained[0])?, true),
            (span(source, retained[1])?, true)
        ]
    );
    assert_eq!(
        properties.omitted,
        vec![
            span(source, omitted[0])?,
            span(source, omitted[1])?,
            span(source, omitted[2])?
        ]
    );
    assert_rendered(&observed.rendered.mapped.code, &retained, &omitted)?;
    Ok(())
}

#[test]
fn proto_named_method_and_shorthand_remain_own_when_a_setter_is_present() -> TestResult {
    // Given
    let source = "const __proto__={space:'24px'};export const tokens={__proto__:{space:'8px'},__proto__,__proto__(){return 'method'},browser:window.document};";
    let method = "__proto__(){return 'method'}";
    let omitted = ["__proto__:{space:'8px'}", "browser:window.document"];
    assert_eq!(source.matches(",__proto__,").count(), 1);
    let delimited = span(source, ",__proto__,")?;
    let delimiter = u32::try_from(",".len())?;
    let shorthand = Span::new(
        delimited
            .start
            .checked_add(delimiter)
            .ok_or("shorthand start overflow")?,
        delimited
            .end
            .checked_sub(delimiter)
            .ok_or("shorthand end underflow")?,
    );
    let demand = path(&["tokens", "__proto__"]);
    // When
    let observed = observe("/proto-own-shapes.ts", source, &demand)?;
    // Then
    let tokens = owner(&observed.view, "tokens")?;
    let local = owner(&observed.view, "__proto__")?;
    assert_eq!(observed.view.selection.units.len(), 2);
    assert_eq!(
        observed.view.units.get(&tokens),
        Some(&path(&["__proto__"]))
    );
    assert_eq!(observed.view.units.get(&local), Some(&Demand::whole()));
    assert_eq!(observed.view.properties.len(), 1);
    let properties = observed
        .view
        .properties
        .get(&tokens)
        .ok_or("missing tokens pruning plan")?;
    assert_eq!(
        properties
            .kept
            .iter()
            .map(|(span, nested)| (*span, nested.is_none()))
            .collect::<Vec<_>>(),
        vec![(shorthand, true), (span(source, method)?, true)]
    );
    assert_eq!(
        properties.omitted,
        vec![span(source, omitted[0])?, span(source, omitted[1])?]
    );
    assert_rendered(
        &observed.rendered.mapped.code,
        &["__proto__={space:'24px'}", "{__proto__,", method],
        &omitted,
    )?;
    Ok(())
}
