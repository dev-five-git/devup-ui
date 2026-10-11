use super::super::plan::{MemberDemand, UnitKind};
use super::{names, selected, text};

#[test]
fn selects_individual_declarators_when_siblings_need_host_or_jsx_state() {
    // Given
    let source = "import {createTheme,style} from '@vanilla-extract/css';import React from 'react';const tokens={space:'8px'},browser=window.document;export const [theme,vars]=createTheme(tokens), view=<div/>;const box=style({margin:vars.space}),runtime=document.title;throw new Error('only runtime');";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["tokens", "theme", "vars", "box"]);
    assert_eq!(plan.roots.len(), 2);
    assert_eq!(plan.imports.len(), 2);
    assert!(
        plan.reads
            .iter()
            .all(|read| !matches!(read.name.as_str(), "window" | "document" | "React"))
    );
    assert!(
        plan.units
            .iter()
            .all(|unit| !text(source, unit.span).contains("browser="))
    );
}

#[test]
fn reserves_excluded_names_when_native_results_need_generated_bindings() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';const css=window.name,globalCss=document.title,keyframes=window.location,_ve0=window.document,__ve_capture_0__=window.innerWidth;style({color:'red'});const base=style({padding:8});export const handler=(argument)=>[css,base,argument];";
    // When
    let plan = selected(source);
    // Then
    for name in [
        "css",
        "globalCss",
        "keyframes",
        "_ve0",
        "__ve_capture_0__",
        "handler",
        "argument",
    ] {
        assert!(plan.reserved_names.contains(name), "{name}");
    }
    assert_eq!(names(&plan.units), vec!["base"]);
}

#[test]
fn includes_unused_initializations_and_standalone_effects_in_source_order() {
    // Given
    let source = "import {createVar,globalStyle,createTheme,style} from '@vanilla-extract/css';const unused=createVar();globalStyle('body',{margin:0});const [theme,vars]=createTheme({space:'8px'});style({margin:vars.space});const host=window.name;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(
        plan.roots
            .iter()
            .map(|root| text(source, root.span))
            .collect::<Vec<_>>(),
        vec![
            "unused=createVar()",
            concat!("globalStyle('body',", "{margin:0});"),
            "[theme,vars]=createTheme({space:'8px'})",
            "style({margin:vars.space});",
        ]
    );
    assert_eq!(
        plan.roots
            .iter()
            .map(|root| root
                .captures
                .iter()
                .map(|binding| binding.name.as_str())
                .collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec!["unused"], vec![], vec!["theme", "vars"], vec![]]
    );
}

#[test]
fn preserves_forward_reads_and_function_hoisting_without_topological_sorting() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';const box=style({padding:later(),margin:size});function later(){return 8}const size=12,host=window.name;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["box", "later", "size"]);
    assert!(matches!(
        plan.units[1].kind,
        UnitKind::Function { erased: false }
    ));
    let forward = plan
        .reads
        .iter()
        .find(|read| read.name == "size")
        .unwrap_or_else(|| panic!("missing forward read"));
    assert_eq!(text(source, forward.span), "size");
    assert!(forward.span.start < plan.units[2].span.start);
}

#[test]
fn keeps_the_entire_pattern_when_defaults_and_nested_native_calls_share_an_owner() {
    // Given
    let source = "import {createVar,style} from '@vanilla-extract/css';function make(){const value=createVar();return {nested:{value},box:style({margin:value})}}const {nested:{value},box,missing=createVar('default')}=make();const root=style([box,{padding:value,borderWidth:missing}]);";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(
        names(&plan.units),
        vec!["make", "value", "box", "missing", "root"]
    );
    assert_eq!(plan.roots.len(), 2);
    assert_eq!(plan.roots[0].native_calls.len(), 3);
    assert_eq!(
        plan.roots[0]
            .captures
            .iter()
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>(),
        vec!["value", "box", "missing"]
    );
    let UnitKind::Declarator { pattern, .. } = plan.units[1].kind else {
        panic!("destructuring owner")
    };
    assert_eq!(
        text(source, pattern),
        "{nested:{value},box,missing=createVar('default')}"
    );
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn retains_original_import_sites_and_narrow_member_demands() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';import * as tokens from './tokens';import unused from 'browser-only';const box=style({color:tokens.colors['bg']});";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.imports.len(), 2);
    let imported = plan
        .imports
        .iter()
        .find(|import| import.source == "./tokens")
        .unwrap_or_else(|| panic!("missing selected import"));
    assert_eq!(text(source, imported.specifier), "* as tokens");
    assert_eq!(
        text(source, imported.declaration),
        "import * as tokens from './tokens';"
    );
    let demand = plan
        .demands
        .iter()
        .find(|demand| demand.symbol == imported.binding.symbol)
        .unwrap_or_else(|| panic!("missing selected demand"));
    assert_eq!(text(source, demand.read), "tokens");
    assert!(matches!(&demand.member, MemberDemand::Path(path) if path == &["colors", "bg"]));
}
