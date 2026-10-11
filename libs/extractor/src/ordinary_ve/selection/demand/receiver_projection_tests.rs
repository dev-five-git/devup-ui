use oxc_span::Span;

use super::receiver_test_support::{TestResult, assert_identity, objects, observe, path, span};
use super::{
    Demand,
    receiver_fixtures::{P1, P2},
};

#[test]
fn receiver_path_is_added_when_a_retained_ordinary_method_reads_wrapped_this() -> TestResult {
    // Given
    let demand = path(&["tokens", "read"]);
    let mut expected = path(&["read"]);
    expected.merge(&path(&["space"]));
    let ty = span(P1, "typeof tokens")?;
    let type_read = Span::new(
        ty.start
            .checked_add(u32::try_from("typeof ".len())?)
            .ok_or("type span overflow")?,
        ty.end,
    );
    let omitted = ["unused(){return this.browser}", "browser:window.document"];
    // When
    let observed = observe("/receiver.ts", P1, &demand)?;
    // Then
    assert_identity(
        &observed,
        P1,
        &[(
            "tokens",
            P1.trim_start_matches("export const ").trim_end_matches(';'),
        )],
    )?;
    let unit = &observed.view.selection.units[0];
    assert_eq!(observed.view.units.get(&unit.node), Some(&expected));
    assert_eq!(observed.view.properties.len(), 1);
    assert_eq!(
        observed
            .view
            .properties
            .get(&unit.node)
            .ok_or("missing pruning")?
            .omitted,
        vec![span(P1, omitted[0])?, span(P1, omitted[1])?]
    );
    assert_eq!(observed.view.selection.imports.len(), 0);
    assert_eq!(observed.view.forwarded, vec![]);
    assert_eq!(observed.type_reads, vec![("tokens".into(), type_read)]);
    assert_eq!(
        objects(&observed.rendered)?,
        vec![(
            "tokens".into(),
            vec![Some("space".into()), Some("read".into())]
        )]
    );
    assert!(
        observed
            .rendered
            .mapped
            .code
            .contains("read(){return ((this as typeof tokens)!).space}")
    );
    for property in omitted {
        assert!(!observed.rendered.mapped.code.contains(property));
    }
    Ok(())
}

#[test]
fn receiver_becomes_whole_when_a_retained_ordinary_method_returns_this() -> TestResult {
    // Given
    let demand = path(&["tokens", "read"]);
    // When
    let observed = observe("/receiver.ts", P2, &demand)?;
    // Then
    assert_identity(
        &observed,
        P2,
        &[(
            "tokens",
            P2.trim_start_matches("export const ").trim_end_matches(';'),
        )],
    )?;
    let unit = &observed.view.selection.units[0];
    assert_eq!(observed.view.units.get(&unit.node), Some(&Demand::whole()));
    assert!(!observed.view.properties.contains_key(&unit.node));
    assert_eq!(observed.view.properties.len(), 0);
    assert_eq!(observed.view.selection.imports.len(), 0);
    assert_eq!(observed.view.forwarded, vec![]);
    assert_eq!(
        objects(&observed.rendered)?,
        vec![(
            "tokens".into(),
            vec![
                Some("space".into()),
                Some("read".into()),
                Some("browser".into())
            ]
        )]
    );
    assert!(
        observed
            .rendered
            .mapped
            .code
            .contains("read(){return this}")
    );
    assert!(
        observed
            .rendered
            .mapped
            .code
            .contains("browser:window.document")
    );
    Ok(())
}
