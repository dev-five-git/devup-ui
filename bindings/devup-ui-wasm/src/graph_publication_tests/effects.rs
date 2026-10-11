use super::support::*;
use super::*;

#[rstest]
#[serial]
fn producer_effects_when_diamond_is_extracted_reach_original_sheet_owners(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given: control extraction establishes identities but never publishes to the global sheet.
    setup();
    let graph = native(suffix, single);
    let expected = names(&graph)?;
    // When
    let output = graph.extract()?;
    // Then: these are actual consumer-graph publications, not the control's style set.
    let entry = bucket(if single { None } else { Some(&graph.entry) });
    assert_eq!(output.css(), Some(entry.clone()));
    for declaration in ["color:blue", "padding:8px", "z-index:5"] {
        assert!(entry.contains(declaration), "{entry}");
    }
    assert!(
        entry.contains(&format!("margin:var({})", expected.variable)),
        "{entry}"
    );
    assert!(
        entry.contains(&format!("animation-name:{}", expected.spin)),
        "{entry}"
    );
    assert!(bucket(None).contains(&format!("{}{{outline-color:purple}}", expected.focus)));
    assert!(!entry.contains("padding:32px"));
    assert!(!output.code().contains("make("));
    assert!(!output.code().contains("@vanilla-extract/css"));
    assert!(output.code().contains("./left") && output.code().contains("./right"));
    assert!(output.code().contains("window.document"));
    for path in [
        &graph.origin,
        &format!("/b-left.{suffix}"),
        &format!("/b-right.{suffix}"),
    ] {
        assert!(output.dependencies().contains(path));
    }
    if !single {
        let number = css::file_map::get_file_num_by_filename(&graph.entry);
        assert!(
            output
                .css_file()
                .is_some_and(|path| path.ends_with(&format!("{number}.css")))
        );
        assert!(!entry.contains("color:red"));
        assert!(!entry.contains("@keyframes"));
        assert!(!entry.contains(&format!("{}:8px", expected.variable)));
    }
    effects(&graph, &expected, ("8px", "purple", "1"));
    Ok(())
}
