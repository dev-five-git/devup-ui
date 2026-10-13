use super::support::*;
use super::*;

#[rstest]
#[case(("import {css,globalCss} from '@devup-ui/react';", "css", "globalCss"))]
#[case(("import * as du from '@devup-ui/react';", "du.css", "du.globalCss"))]
#[serial]
fn fallback_effects_when_native_import_is_unused_reach_actual_owner_bucket(
    #[values(("css.ts", true), ("css.ts", false), ("css.js", true), ("css.js", false))] target: (
        &str,
        bool,
    ),
    #[values(
        "",
        "import {style as unused} from '@vanilla-extract/css';",
        "import * as unused from '@vanilla-extract/css';"
    )]
    unused: &str,
    #[case] api: (&str, &str, &str),
) -> TestResult {
    // Given
    setup();
    let (suffix, single) = target;
    let (import, css, global) = api;
    let origin = format!("/b-fallback.{suffix}");
    let graph = Graph {
        entry: format!("/b-fallback-entry.{suffix}"), origin: origin.clone(), single,
        source: "import {style} from '@vanilla-extract/css';import {base} from './data';export const box=style([base,{margin:3,content:JSON.stringify(base)}]);const browser=window.document;".into(),
        modules: HashMap::from([("./data".into(), ResolvedModule {
            path: origin,
            code: format!("{unused}{import}export const base={css}({{color:'orange',p:2}});{global}({{body:{{borderWidth:'12px'}}}});"),
        })]),
    };
    // When
    let output = graph.extract()?;
    // Then
    let owner = bucket(if single { None } else { Some(&graph.origin) });
    assert!(
        owner.contains("color:orange") && owner.contains("padding:8px"),
        "{owner}"
    );
    assert!(bucket(None).contains("body{border-width:12px}"));
    let entry = bucket(if single { None } else { Some(&graph.entry) });
    assert!(
        entry.contains("margin:3px") && entry.contains("content:"),
        "{entry}"
    );
    assert!(!entry.contains("padding:32px"));
    assert!(output.dependencies().contains(&graph.origin));
    assert!(output.code().contains("./data"));
    assert!(!output.code().contains("style("));
    if !single {
        assert!(!entry.contains("color:orange") && !entry.contains("padding:8px"));
    }
    Ok(())
}

#[rstest]
#[serial]
fn nested_fallback_effects_when_parent_retains_child_are_published_once(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let child = format!("/b-fallback-child.{suffix}");
    let parent = format!("/b-fallback-parent.{suffix}");
    let graph = Graph {
        entry: format!("/b-nested-entry.{suffix}"), origin: parent.clone(), single,
        source: concat!("import {style} from '@vanilla-extract/css';", "import {base} from './parent';export const box=style([base,{margin:3}]);const browser=window.document;").into(),
        modules: HashMap::from([
            ("./parent".into(), ResolvedModule { path: parent.clone(), code: "import {css,globalCss} from '@devup-ui/react';import {base as child} from './child';export const base=child+' '+css({width:'17px'});globalCss({html:{borderWidth:'19px'}});".into() }),
            ("./child".into(), ResolvedModule { path: child.clone(), code: "import {css,globalCss} from '@devup-ui/react';export const base=css({color:'orange',p:2});globalCss({body:{borderWidth:'12px'}});".into() }),
        ]),
    };
    // When
    let output = graph.extract()?;
    // Then
    let child_css = bucket(if single { None } else { Some(&child) });
    let parent_css = bucket(if single { None } else { Some(&parent) });
    assert_eq!(child_css.matches("color:orange").count(), 1, "{child_css}");
    assert!(child_css.contains("padding:8px"));
    assert_eq!(parent_css.matches("width:17px").count(), 1, "{parent_css}");
    let global = bucket(None);
    assert_eq!(
        global.matches("body{border-width:12px}").count(),
        1,
        "{global}"
    );
    assert_eq!(
        global.matches("html{border-width:19px}").count(),
        1,
        "{global}"
    );
    assert!(output.dependencies().contains(&child) && output.dependencies().contains(&parent));
    Ok(())
}
