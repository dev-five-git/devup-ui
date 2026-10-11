use super::support::*;
use super::*;

#[rstest]
#[serial]
fn current_effects_when_same_owner_source_changes_replace_previous_payload(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
    #[values(false, true)] another_entry: bool,
) -> TestResult {
    // Given
    setup();
    let mut graph = native(suffix, single);
    graph.extract()?;
    let changed = format!(
        "{}const make=ve.style;export {{make}};const browser=window.document;",
        roots(("13px", "teal", "0.25"))
    );
    graph.replace_origin(&changed);
    if another_entry {
        graph.entry = format!("/b-second.{suffix}");
    }
    let expected = names_for(&graph, ("13px", "teal", "0.25"))?;
    // When: no sheet, file/class map or imported-run metadata is cleared.
    let output = graph.extract()?;
    // Then
    effects(&graph, &expected, ("13px", "teal", ".25"));
    let global = bucket(None);
    assert!(!global.contains(&format!("{}{{{}:8px}}", expected.theme, expected.variable)));
    assert!(!global.contains(concat!("body", "{color:purple}")));
    let entry = bucket(if single { None } else { Some(&graph.entry) });
    assert!(entry.contains("z-index:5"));
    assert!(
        entry.contains(&format!("animation-name:{}", expected.spin)),
        "{entry}"
    );
    assert_eq!(
        entry
            .matches(&format!("{{animation-name:{}}}", expected.spin))
            .count(),
        1,
        "{entry}"
    );
    if single {
        assert_eq!(output.css(), Some(entry));
        assert!(output.updated_base_style());
    } else {
        assert_eq!(output.css(), Some(entry.clone()));
        assert!(!entry.contains("@keyframes"), "{entry}");
    }
    Ok(())
}

#[rstest]
#[serial]
fn current_effects_when_failed_same_path_graph_is_corrected_are_published(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let mut graph = native(suffix, single);
    graph.extract()?;
    let before = export_sheet_internal()?;
    let failed = format!(
        "{}const make=ve.style;export {{make}};",
        roots(("21px", "salmon", "0.75"))
    );
    graph.replace_origin(&failed.replace("space:'21px'", "space:window.name"));
    let error = graph
        .extract()
        .err()
        .ok_or("required producer input succeeded")?;
    assert!(
        error.contains("cannot use `window.name` at build time"),
        "{error}"
    );
    assert!(error.contains(&format!("{}:", graph.origin)), "{error}");
    assert_eq!(export_sheet_internal()?, before);
    graph.replace_origin(&format!(
        "{}const make=ve.style;export {{make}};",
        roots(("17px", "navy", "0.5"))
    ));
    let expected = names_for(&graph, ("17px", "navy", "0.5"))?;
    // When
    graph.extract()?;
    // Then
    effects(&graph, &expected, ("17px", "navy", ".5"));
    assert!(!bucket(None).contains(concat!("body", "{color:salmon}")));
    let entry = bucket(if single { None } else { Some(&graph.entry) });
    assert!(
        entry.contains(&format!("animation-name:{}", expected.spin)),
        "{entry}"
    );
    assert_eq!(
        entry
            .matches(&format!("{{animation-name:{}}}", expected.spin))
            .count(),
        1,
        "{entry}"
    );
    if !single {
        assert!(!entry.contains("@keyframes"), "{entry}");
    }
    Ok(())
}
