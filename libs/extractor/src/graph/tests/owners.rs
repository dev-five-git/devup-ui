use super::*;

#[rstest]
#[serial]
fn actual_artifact_when_diamond_demands_one_producer_keeps_owner_and_exact_effects(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-origin.{suffix}");
    let entry = format!("/graph-entry.{suffix}");
    let left = format!("/graph-left.{suffix}");
    let right = format!("/graph-right.{suffix}");
    let source = producer(("8px", "purple", "1"));
    // When
    let graph = run(
        (&entry, ENTRY),
        &[
            ("./left", &left, "export {make,base,count} from './origin';"),
            ("./right", &right, "export {vars,spin} from './origin';"),
            ("./origin", &owner, &source),
        ],
        single,
    )?;
    // Then
    assert_eq!(
        graph
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![right.as_str(), owner.as_str()]
    );
    let cleanup = graph
        .artifacts
        .iter()
        .find(|item| item.filename == right)
        .ok_or("right cleanup artifact absent")?;
    assert_eq!(cleanup.output.styles.len(), 0);
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
        .ok_or("producer artifact absent")?;
    payload(&artifact.output, &owner, ("8px", "purple", "1"));
    let spin = artifact
        .output
        .styles
        .iter()
        .find(|style| matches!(style, ExtractStyleValue::Keyframes(_)))
        .and_then(|style| style.extract(if single { None } else { Some(&owner) }))
        .ok_or("producer keyframes absent")?
        .to_string();
    assert!(has(&graph.entry, "animation-name", &spin));
    assert!(has(&graph.entry, "color", "blue") && has(&graph.entry, "z-index", "5"));
    assert!(!has(&graph.entry, "color", "purple"));
    assert!(
        !graph
            .entry
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Keyframes(_)))
    );
    Ok(())
}

#[rstest]
#[serial]
fn rootless_carrier_when_only_api_is_demanded_creates_no_artifact(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-rootless.{suffix}");
    // When
    let graph = run(
        (
            "/graph-rootless-entry.tsx",
            "import {make} from './api';export const box=make({color:'blue'});const browser=window.document;",
        ),
        &[(
            "./api",
            &owner,
            "export {style as make} from '@vanilla-extract/css';",
        )],
        single,
    )?;
    // Then
    assert_eq!(graph.artifacts.len(), 0);
    assert!(has(&graph.entry, "color", "blue"));
    assert!(graph.entry.code.contains("./api"));
    Ok(())
}

#[rstest]
#[serial]
fn nested_fallback_when_two_paths_share_child_keeps_completion_order_once(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let child = format!("/graph-child.{suffix}");
    let parent = format!("/graph-parent.{suffix}");
    let entry = format!("/graph-fallback-entry.{suffix}");
    // When
    let graph = run(
        (
            &entry,
            "import {style} from '@vanilla-extract/css';import {base} from './parent';import {base as child} from './child';export const box=style({margin:3,content:base+child});const browser=window.document;",
        ),
        &[
            (
                "./parent",
                &parent,
                "import {css,globalCss} from '@devup-ui/react';import {base as child} from './child';export const base=child+' '+css({width:'17px'});globalCss({html:{borderWidth:'19px'}});",
            ),
            (
                "./child",
                &child,
                "import {css,globalCss} from '@devup-ui/react';export const base=css({color:'orange',p:2});globalCss({body:{borderWidth:'12px'}});",
            ),
        ],
        single,
    )?;
    // Then
    assert_eq!(
        graph
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![child.as_str(), parent.as_str()]
    );
    let child_output = &graph.artifacts.first().ok_or("child absent")?.output;
    let parent_output = &graph.artifacts.last().ok_or("parent absent")?.output;
    assert!(has(child_output, "color", "orange") && has(child_output, "padding", "8px"));
    assert!(has(parent_output, "width", "17px") && has(parent_output, "border-width", "19px"));
    assert!(!has(&graph.entry, "padding", "8px"));
    Ok(())
}

#[rstest]
#[serial]
fn artifacts_when_raw_owners_share_canonical_bucket_remain_distinct(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let first = format!("/graph-first.{suffix}");
    let second = format!("/graph-second.{suffix}");
    css::file_map::set_canonical_map(std::collections::HashMap::from([
        (first.clone(), "/graph-bucket.tsx".to_string()),
        (second.clone(), "/graph-bucket.tsx".to_string()),
    ]));
    // When
    let graph = run(
        (
            "/graph-canonical.tsx",
            "import {style} from '@vanilla-extract/css';import {base as a} from './a';import {base as b} from './b';export const box=style({content:a+b});const browser=window.document;",
        ),
        &[
            (
                "./a",
                &first,
                "import {css} from '@devup-ui/react';export const base=css({color:'orange'});",
            ),
            (
                "./b",
                &second,
                "import {css} from '@devup-ui/react';export const base=css({width:'17px'});",
            ),
        ],
        single,
    )?;
    // Then
    assert_eq!(
        graph
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![first.as_str(), second.as_str()]
    );
    Ok(())
}

#[rstest]
#[serial]
fn native_owner_when_its_root_reads_fallback_waits_for_child_artifact(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let child = format!("/graph-mixed-child.{suffix}");
    let owner = format!("/graph-mixed-native.{suffix}");
    // When
    let graph = run(
        (
            "/graph-mixed-entry.tsx",
            "import {make,base,count} from './origin';export const box=make([base,{color:'blue',zIndex:count()}]);const browser=window.document;",
        ),
        &[
            (
                "./origin",
                &owner,
                "import * as ve from '@vanilla-extract/css';import {base as child} from './child';let runs=0;export const base=(runs++,ve.style({content:child,color:'red'}));(runs++,ve.globalStyle('body',{color:'purple'}));export function count(){return runs;}export const make=ve.style;const browser=window.document;",
            ),
            (
                "./child",
                &child,
                "import {css} from '@devup-ui/react';export const base=css({color:'orange',p:2});",
            ),
        ],
        single,
    )?;
    // Then
    assert_eq!(
        graph
            .artifacts
            .iter()
            .map(|item| item.filename.as_str())
            .collect::<Vec<_>>(),
        vec![child.as_str(), owner.as_str()]
    );
    assert!(has(
        &graph.artifacts.first().ok_or("mixed child absent")?.output,
        "padding",
        "8px"
    ));
    assert!(has(
        &graph.artifacts.last().ok_or("native owner absent")?.output,
        "color",
        "purple"
    ));
    assert!(has(&graph.entry, "z-index", "2"));
    Ok(())
}
