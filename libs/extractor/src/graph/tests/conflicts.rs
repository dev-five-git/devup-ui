use super::*;

#[rstest]
#[serial]
fn graph_when_nested_fallbacks_compile_conflicting_raw_owner_payloads_refuses_union(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    setup();
    let owner = format!("/graph-conflict-child.{suffix}");
    let left = format!("/graph-conflict-left.{suffix}");
    let right = format!("/graph-conflict-right.{suffix}");
    let expected = owner.clone();
    let resolver = move |specifier: &str, importer: &str| {
        match specifier {
        "./left" => Some(ResolvedModule {
            path: left.clone(),
            code: "import {css} from '@devup-ui/react';import {base as child} from './child';export const base=child+' '+css({width:'17px'});".into(),
        }),
        "./right" => Some(ResolvedModule {
            path: right.clone(),
            code: "import {css} from '@devup-ui/react';import {base as child} from './child';export const base=child+' '+css({height:'19px'});".into(),
        }),
        "./child" => Some(ResolvedModule {
            path: owner.clone(),
            code: format!("import {{css}} from '@devup-ui/react';export const base=css({{color:'{}'}});", if importer == left { "red" } else { "blue" }),
        }),
        _ => None,
    }
    };
    // When
    let error = extract_graph("/graph-conflict-entry.tsx", "import {style} from '@vanilla-extract/css';import {base as a} from './left';import {base as b} from './right';export const box=style({content:a+b});const browser=window.document;",
        option(single), false, Some(&resolver)).err().ok_or("conflicting owner silently succeeded")?;
    // Then
    assert!(
        error.to_string().contains(&format!("{expected}:1:1:")),
        "{error}"
    );
    assert!(
        error.to_string().contains("incompatible compiled outputs"),
        "{error}"
    );
    Ok(())
}
