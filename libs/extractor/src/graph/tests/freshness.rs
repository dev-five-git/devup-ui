use super::*;

#[rstest]
#[serial]
fn current_graph_when_same_path_changes_keeps_previous_returned_graph_immutable(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-fresh.{suffix}");
    let left = format!("/graph-fresh-left.{suffix}");
    let right = format!("/graph-fresh-right.{suffix}");
    let original = producer(("8px", "purple", "1"));
    let modules = |source: &str| {
        vec![
            (
                "./left".to_string(),
                left.clone(),
                "export {make,base,count} from './origin';".to_string(),
            ),
            (
                "./right".to_string(),
                right.clone(),
                "export {vars,spin} from './origin';".to_string(),
            ),
            ("./origin".to_string(), owner.clone(), source.to_string()),
        ]
    };
    let first_modules = modules(&original);
    let first: Vec<_> = first_modules
        .iter()
        .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
        .collect();
    let previous = run(("/graph-fresh-entry.tsx", ENTRY), &first, single)?;
    let changed = producer(("13px", "teal", "0.25"));
    let changed_modules = modules(&changed);
    let changed: Vec<_> = changed_modules
        .iter()
        .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
        .collect();
    // When: no allocation maps or imported-run metadata are cleared.
    let current = run(("/graph-fresh-entry.tsx", ENTRY), &changed, single)?;
    // Then
    assert_eq!(
        current
            .artifacts
            .iter()
            .filter(|item| item.filename == owner)
            .count(),
        1
    );
    assert_eq!(
        current
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![right.as_str(), owner.as_str()]
    );
    let current_cleanup = current
        .artifacts
        .iter()
        .find(|item| item.filename == right)
        .ok_or("current right cleanup absent")?;
    assert_eq!(current_cleanup.output.styles.len(), 0);
    let current_owner = current
        .artifacts
        .iter()
        .find(|item| item.filename == owner)
        .ok_or("current owner absent")?;
    payload(&current_owner.output, &owner, ("13px", "teal", ".25"));
    assert!(!has(&current_owner.output, "color", "purple"));
    assert_eq!(
        previous
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![right.as_str(), owner.as_str()]
    );
    assert_eq!(
        previous
            .artifacts
            .iter()
            .filter(|item| item.filename == owner)
            .count(),
        1
    );
    let previous_cleanup = previous
        .artifacts
        .iter()
        .find(|item| item.filename == right)
        .ok_or("previous right cleanup absent")?;
    assert_eq!(previous_cleanup.output.styles.len(), 0);
    let previous_owner = previous
        .artifacts
        .iter()
        .find(|item| item.filename == owner)
        .ok_or("previous owner absent")?;
    payload(&previous_owner.output, &owner, ("8px", "purple", "1"));
    assert!(!has(&previous_owner.output, "color", "teal"));
    let current_spin = current_owner
        .output
        .styles
        .iter()
        .find(|style| matches!(style, ExtractStyleValue::Keyframes(_)))
        .and_then(|style| style.extract(if single { None } else { Some(&owner) }))
        .ok_or("current keyframes absent")?
        .to_string();
    let previous_spin = previous_owner
        .output
        .styles
        .iter()
        .find(|style| matches!(style, ExtractStyleValue::Keyframes(_)))
        .and_then(|style| style.extract(if single { None } else { Some(&owner) }))
        .ok_or("previous keyframes absent")?
        .to_string();
    assert_ne!(current_spin, previous_spin);
    assert!(has(&current.entry, "animation-name", &current_spin));
    assert!(!has(&current.entry, "animation-name", &previous_spin));
    assert!(has(&previous.entry, "animation-name", &previous_spin));
    assert!(!has(&previous.entry, "animation-name", &current_spin));
    Ok(())
}

#[rstest]
#[serial]
fn graph_when_consumer_fails_after_owner_finishes_returns_only_error(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-failed.{suffix}");
    let source = producer(("8px", "purple", "1"));
    // When
    let error = run(("/graph-failed-entry.tsx", "import {make,base} from './origin';export const box=make([base,{color:window.name}]);const browser=window.document;"),
        &[("./origin", &owner, &source)], single).err().ok_or("partial graph succeeded")?;
    // Then
    assert!(error.to_string().contains("/graph-failed-entry.tsx:"));
    assert!(
        error
            .to_string()
            .contains("cannot use `window.name` at build time")
    );
    Ok(())
}

#[rstest]
#[serial]
fn graph_when_failed_owner_source_is_corrected_returns_only_current_payload(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-correction.{suffix}");
    let entry = "import {make,vars,spin} from './origin';export const box=make({margin:vars.space,animationName:spin});const browser=window.document;";
    let original = producer(("8px", "purple", "1"));
    run(
        ("/graph-correction-entry.tsx", entry),
        &[("./origin", &owner, &original)],
        single,
    )?;
    let invalid = producer(("21px", "salmon", "0.75")).replace("space:'21px'", "space:window.name");
    let failed = run(
        ("/graph-correction-entry.tsx", entry),
        &[("./origin", &owner, &invalid)],
        single,
    )
    .err()
    .ok_or("required invalid input succeeded")?;
    assert!(
        failed
            .to_string()
            .contains("cannot use `window.name` at build time")
    );
    let corrected = producer(("17px", "navy", "0.5"));
    // When: the same paths and existing allocation metadata are retained.
    let graph = run(
        ("/graph-correction-entry.tsx", entry),
        &[("./origin", &owner, &corrected)],
        single,
    )?;
    // Then
    assert_eq!(graph.artifacts.len(), 1);
    assert_eq!(
        graph
            .artifacts
            .iter()
            .filter(|item| item.filename == owner)
            .count(),
        1
    );
    let artifact = graph
        .artifacts
        .iter()
        .find(|item| item.filename == owner)
        .ok_or("corrected owner absent")?;
    payload(&artifact.output, &owner, ("17px", "navy", ".5"));
    let spin = artifact
        .output
        .styles
        .iter()
        .find(|style| matches!(style, ExtractStyleValue::Keyframes(_)))
        .and_then(|style| style.extract(if single { None } else { Some(&owner) }))
        .ok_or("corrected keyframes absent")?
        .to_string();
    assert!(has(&graph.entry, "animation-name", &spin));
    assert!(!has(&artifact.output, "color", "salmon"));
    Ok(())
}
