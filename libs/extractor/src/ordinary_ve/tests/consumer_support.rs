use crate::ExtractOutput;

use super::demand_support::{assert_import, run, static_value};
use super::mixed_support::{assert_consumed, has_static};

pub(super) const TOKENS: &str = "export const PRIMARY='blue',browser=window.document;export function space(n){return n*2+'px'};throw new Error('token runtime only');";
pub(super) const PRODUCER: &str = "import {createVar,createTheme,style,keyframes,globalStyle} from '@vanilla-extract/css';import {PRIMARY,space} from './tokens';import './reset.css';createVar();export const [theme,vars]=createTheme({space:space(4)});export const base=style({color:'red',padding:8});export const spin=keyframes({to:{opacity:1}});globalStyle('body',{color:'purple'});export const COLORS={fg:PRIMARY,unused:window.document};export default vars;export const handler=()=>document.title;throw new Error('producer runtime only');";
const CONTROL: &str = "import {createVar,createTheme,style,keyframes,globalStyle} from '@vanilla-extract/css';import './reset.css';createVar();export const [theme,vars]=createTheme({space:'8px'});export const base=style({color:'red',padding:8});export const spin=keyframes({to:{opacity:1}});globalStyle('body',{color:'purple'});export const check=style({margin:vars.space,animationName:spin});globalStyle(`${base}:focus`,{outlineColor:'purple'});";

pub(super) struct Owner {
    pub margin: String,
    pub spin: String,
    pub base_selector: String,
}

pub(super) fn owner(path: &str) -> Result<Owner, Box<dyn std::error::Error>> {
    let output = run(
        path,
        CONTROL,
        &[("./reset.css", "/reset.css", "body{margin:0}")],
    )?;
    let margin = static_value(&output, "margin").to_string();
    assert!(margin.starts_with("var(--space-"), "{margin}");
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    let variable = margin
        .strip_prefix("var(")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or("control variable missing")?;
    assert!(has_static(&output, variable, "8px"), "{:?}", output.styles);
    Ok(Owner {
        margin,
        spin: static_value(&output, "animation-name").to_string(),
        base_selector: super::demand_support::global_selector(&output, "outline-color"),
    })
}

pub(super) fn modules(path: &str) -> [(&str, &str, &str); 3] {
    [
        ("./producer", path, PRODUCER),
        ("./tokens", "/tokens.ts", TOKENS),
        ("./reset.css", "/reset.css", "body{margin:0}"),
    ]
}

pub(super) fn exact(output: &ExtractOutput, expected: &Owner) {
    assert_consumed(output);
    assert!(
        has_static(output, "margin", &expected.margin),
        "{:?}",
        output.styles
    );
    assert!(!output.code.contains("__style_"), "{}", output.code);
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    assert!(!output.code.contains("COLORS.fg"), "{}", output.code);
}

pub(super) fn edges(output: &ExtractOutput, paths: &[&str]) {
    assert_import(output, "./producer");
    for path in paths {
        assert!(
            output
                .dependencies
                .iter()
                .any(|dependency| dependency == path),
            "{path}: {:?}",
            output.dependencies
        );
    }
}

pub(super) fn located_failure(
    result: Result<ExtractOutput, Box<dyn std::error::Error>>,
    place: &str,
    cause: &str,
) {
    let error = match result {
        Ok(output) => panic!(
            "required failure published output: {} {:?}",
            output.code, output.styles
        ),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(place), "{error}");
    assert!(error.contains(cause), "{error}");
    if cause.starts_with("cannot use `") {
        assert!(error.contains("exact, static input"), "{error}");
    }
    assert!(error.contains("Fix:"), "{error}");
}
