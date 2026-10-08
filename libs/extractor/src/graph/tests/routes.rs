use super::*;

#[rstest]
#[serial]
fn readback_when_element_reads_theme_retains_real_native_owner_artifact(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-readback.{suffix}");
    // When
    let graph = run(
        (
            "/graph-readback-entry.tsx",
            "import {Box} from '@devup-ui/react';import {vars} from './origin';export const node=<Box m={vars.space}/>;const browser=window.document;",
        ),
        &[(
            "./origin",
            &owner,
            "import {createTheme,globalStyle} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});globalStyle('body',{color:'purple'});const browser=window.document;",
        )],
        single,
    )?;
    // Then
    assert_eq!(graph.artifacts.len(), 1);
    let artifact = graph.artifacts.first().ok_or("readback owner absent")?;
    assert_eq!(artifact.filename, owner);
    assert!(has(&artifact.output, "color", "purple"));
    assert!(
        artifact
            .output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Static(style)
        if style.property.starts_with("--space-") && style.value == "8px"))
    );
    assert!(!has(&graph.entry, "color", "purple"));
    assert!(
        graph
            .entry
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Static(style)
        if style.property == "margin" && style.value.starts_with("var(--space-")))
    );
    assert!(graph.entry.code.contains("window.document"));
    Ok(())
}

#[rstest]
#[serial]
fn full_stylesheet_when_computed_export_uses_child_keeps_child_and_entry_separate(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let child = format!("/graph-full-child.{suffix}");
    let entry = format!("/graph-full-entry.{suffix}");
    // When
    let graph = run(
        (
            &entry,
            "import {css} from '@devup-ui/react';import {base as child} from './child';export const base=child+' '+css({width:'17px'});",
        ),
        &[(
            "./child",
            &child,
            "import {css,globalCss} from '@devup-ui/react';export const base=css({color:'orange',p:2});globalCss({body:{borderWidth:'12px'}});",
        )],
        single,
    )?;
    // Then
    assert_eq!(
        graph
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![child.as_str()]
    );
    assert!(has(
        &graph.artifacts.first().ok_or("full child absent")?.output,
        "padding",
        "8px"
    ));
    assert!(has(&graph.entry, "width", "17px"));
    assert!(!has(&graph.entry, "padding", "8px"));
    assert!(!graph.entry.code.contains("css("));
    Ok(())
}

#[rstest]
#[serial]
fn unchanged_dependency_when_output_is_empty_still_returns_raw_cleanup_artifact(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-empty.{suffix}");
    // When
    let graph = run(
        (
            "/graph-empty-entry.tsx",
            concat!(
                "import {style} from '@vanilla-extract/css';",
                "import {base} from './origin';export const box=style({content:base});const browser=window.document;"
            ),
        ),
        &[("./origin", &owner, "export const base='same';")],
        single,
    )?;
    // Then
    assert_eq!(graph.artifacts.len(), 1);
    let artifact = graph
        .artifacts
        .first()
        .ok_or("empty cleanup owner absent")?;
    assert_eq!(artifact.filename, owner);
    assert_eq!(artifact.output.styles.len(), 0);
    assert!(artifact.output.code.contains("same"));
    Ok(())
}
