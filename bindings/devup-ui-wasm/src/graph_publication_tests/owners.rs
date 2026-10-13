use super::support::*;
use super::*;

#[rstest]
#[serial]
fn producer_cleanup_when_canonical_bucket_is_shared_preserves_other_raw_owner(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let root = format!("/b-owner-root.{suffix}");
    let producer = format!("/b-owner-child.{suffix}");
    import_canonical_map_internal(HashMap::from([(producer.clone(), root.clone())]));
    code_extract_internal(
        &root,
        "import {globalCss} from '@devup-ui/react';globalCss({'#root':{color:'gold'}});",
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        aliases(),
    )?;
    let mut graph = Graph {
        entry: format!("/b-owner-entry.{suffix}"), origin: producer, single,
        source: "import {style} from '@vanilla-extract/css';import {base} from './origin';export const box=style({content:base,color:'blue'});const browser=window.document;".into(),
        modules: HashMap::new(),
    };
    graph.replace_origin("import {css,globalCss} from '@devup-ui/react';export const base=css({width:'17px'});globalCss({'#producer':{color:'purple'}});");
    graph.extract()?;
    graph.replace_origin("import {css,globalCss} from '@devup-ui/react';export const base=css({width:'17px'});globalCss({'#producer':{color:'teal'}});");
    // When
    let output = graph.extract()?;
    // Then
    let global = bucket(None);
    assert_eq!(
        global.matches(concat!("#root", "{color:gold}")).count(),
        1,
        "{global}"
    );
    assert_eq!(
        global.matches(concat!("#producer", "{color:teal}")).count(),
        1,
        "{global}"
    );
    assert!(
        !global.contains(concat!("#producer", "{color:purple}")),
        "{global}"
    );
    assert!(output.updated_base_style());
    assert!(bucket(if single { None } else { Some(&root) }).contains("width:17px"));
    Ok(())
}

#[rstest]
#[serial]
fn mixed_global_graph_when_only_one_owner_is_hoisted_uses_each_owners_flag(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(false, true)] producer_global: bool,
) -> TestResult {
    // Given
    setup();
    let producer = format!("/b-global-producer.{suffix}");
    let entry = format!("/b-global-entry.{suffix}");
    let hoisted = if producer_global { &producer } else { &entry };
    import_canonical_map_internal(HashMap::from([(
        hoisted.clone(),
        css::file_map::GLOBAL_BUCKET.to_string(),
    )]));
    let graph = Graph {
        entry: entry.clone(), origin: producer.clone(), single: false,
        source: "import {style} from '@vanilla-extract/css';import {base} from './origin';export const box=style({content:base,color:'blue'});const browser=window.document;".into(),
        modules: HashMap::from([("./origin".into(), ResolvedModule { path: producer.clone(), code: "import {css} from '@devup-ui/react';export const base=css({color:'orange'});".into() })]),
    };
    // When
    let output = graph.extract()?;
    // Then
    let global = bucket(None);
    let local = bucket(Some(if producer_global { &entry } else { &producer }));
    assert!(
        global.contains(if producer_global {
            "color:orange"
        } else {
            "color:blue"
        }),
        "{global}"
    );
    assert!(
        !global.contains(if producer_global {
            "color:blue"
        } else {
            "color:orange"
        }),
        "{global}"
    );
    assert!(
        local.contains(if producer_global {
            "color:blue"
        } else {
            "color:orange"
        }),
        "{local}"
    );
    assert_eq!(
        output.css(),
        Some(bucket(if producer_global { Some(&entry) } else { None }))
    );
    Ok(())
}

#[rstest]
#[serial]
fn single_css_when_only_artifact_raw_css_changes_regenerates_returned_sheet(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    setup();
    let mut graph = Graph {
        entry: format!("/b-base-only-entry.{suffix}"), origin: format!("/b-base-only-origin.{suffix}"), single: true,
        source: concat!("import {style} from '@vanilla-extract/css';", "import {base} from './origin';export const box=style({content:base});const browser=window.document;").into(),
        modules: HashMap::new(),
    };
    graph.replace_origin(concat!(
        "import {globalCss} from '@devup-ui/react';",
        "globalCss`body{color:purple}`;export const base='same';"
    ));
    graph.extract()?;
    graph.replace_origin(concat!(
        "import {globalCss} from '@devup-ui/react';",
        "globalCss`body{color:teal}`;export const base='same';"
    ));
    // When
    let output = graph.extract()?;
    // Then
    let css = output
        .css()
        .ok_or("artifact-only update did not regenerate Output.css")?;
    assert_eq!(css, bucket(None));
    assert!(
        css.contains(concat!("body", "{color:teal}"))
            && !css.contains(concat!("body", "{color:purple}")),
        "{css}"
    );
    assert!(output.updated_base_style());
    Ok(())
}

#[rstest]
#[serial]
fn empty_fallback_artifact_when_effect_is_removed_cleans_its_raw_owner(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let mut graph = Graph {
        entry: format!("/b-empty-entry.{suffix}"), origin: format!("/b-empty-origin.{suffix}"), single,
        source: concat!("import {style} from '@vanilla-extract/css';", "import {base} from './origin';export const box=style({content:base});const browser=window.document;").into(),
        modules: HashMap::new(),
    };
    graph.replace_origin("import {globalCss} from '@devup-ui/react';globalCss({'#removed':{color:'purple'}});export const base='same';");
    graph.extract()?;
    graph.replace_origin("export const base='same';");
    // When
    let output = graph.extract()?;
    // Then
    assert!(!bucket(None).contains(concat!("#removed", "{color:purple}")));
    assert!(output.updated_base_style());
    assert!(output.code().contains("./origin"));
    Ok(())
}
