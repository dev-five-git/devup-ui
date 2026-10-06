use crate::{ExtractOption, ImportAlias, ResolvedModule, extract_with_modules};
use rstest::rstest;
use serial_test::serial;

mod demand_lifecycle;
mod demand_producers;
mod demand_prototype;
mod demand_support;
mod demand_views;
mod mixed_aliases;
mod mixed_apis;
mod mixed_canonical;
mod mixed_gate;
mod mixed_graph;
mod mixed_imports;
mod mixed_review;
mod mixed_rewrite;
mod mixed_selection;
mod mixed_support;

fn option() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from_iter([(
            "@vanilla-extract/css".into(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

#[rstest]
#[case(
    "import {style} from '@vanilla-extract/css';export function render(){return style({color:'red'});}"
)]
#[case("import {style} from '@vanilla-extract/css';export const render=()=>style({color:'red'});")]
#[case(
    "import {createTheme} from '@vanilla-extract/css';const untouched=window.document;export const [theme,vars]=createTheme({space:'8px'});"
)]
#[case(
    "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const view=<div/>;"
)]
fn full_style_evaluation_is_not_selected_when_unrelated_runtime_code_exists(#[case] source: &str) {
    assert!(!super::is_module("/mixed.tsx", source));
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[serial]
fn ordinary_contract_reads_compile_when_the_module_is_static_style_data(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    let code = "import {createThemeContract,createTheme,style} from '@vanilla-extract/css';const vars=createThemeContract({colors:{bg:null}});export const light=createTheme(vars,{colors:{bg:'white'}});export const box=style({background:vars.colors.bg});";
    let output = extract_with_modules(
        &format!("/ordinary.{extension}"),
        code,
        option(),
        false,
        &|_, _| None,
    )?;
    assert!(!output.code.contains("@vanilla-extract/css"));
    assert!(output.code.contains("theme-0-1"));
    assert!(output.styles.iter().any(|value| matches!(value, crate::ExtractStyleValue::Static(style) if style.property()=="background" && style.value()=="var(--colors-bg-0-0)")));
    Ok(())
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[serial]
fn ordinary_imports_preserve_producer_vars_and_d1(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    let resolver = |source: &str, _: &str| {
        match source {
        "./theme.css" => Some(ResolvedModule{path:"/theme.css.ts".into(),code:"import {createTheme,style} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const base=style({color:'red',padding:8});".into()}),
        "./tokens" => Some(ResolvedModule{path:"/tokens.ts".into(),code:"export const PRIMARY='blue';".into()}),
        _ => None,
    }
    };
    let output = extract_with_modules(
        &format!("/ordinary.{extension}"),
        "import {style} from '@vanilla-extract/css';import {base,vars} from './theme.css';import {PRIMARY} from './tokens';export const button=style([base,{color:PRIMARY,margin:vars.space}]);",
        option(),
        false,
        &resolver,
    )?;
    assert!(!output.code.contains("@vanilla-extract/css"));
    assert!(output.styles.iter().any(|value| matches!(value, crate::ExtractStyleValue::Static(style) if style.property()=="margin" && style.value()=="var(--space-0-1)")));
    assert!(!output.styles.iter().any(|value| matches!(value, crate::ExtractStyleValue::Static(style) if style.property()=="color" && style.value()=="red")));
    Ok(())
}
