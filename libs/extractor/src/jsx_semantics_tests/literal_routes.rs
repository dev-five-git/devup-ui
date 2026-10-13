use super::*;
use serial_test::serial;

fn compat(source: &str) -> Result<ExtractOutput, String> {
    compile_with(
        source,
        ExtractOption {
            import_aliases: HashMap::from([
                ("@emotion/css".to_string(), ImportAlias::NamedToNamed),
                ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
                (
                    "@emotion/styled".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
                (
                    "styled-components".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
                (
                    "@vanilla-extract/css".to_string(),
                    ImportAlias::NamedToNamed,
                ),
            ]),
            ..ExtractOption::default()
        },
    )
}

#[test]
#[serial]
fn literal_metadata_when_compat_class_routes_are_aliased_selects_layers() {
    for source in [
        "import {css} from '@emotion/css'; const a=css`style-order:2;color:red`;",
        "import {css} from '@emotion/react'; const a=css('style-order:2;color:red');",
        "import styled from '@emotion/styled'; const a=styled.div`style-order:2;color:red`;",
        "import styled,{css} from 'styled-components'; const a=styled.div`style-order:2;color:red`; const b=css`style-order:3;color:blue`;",
        "import {style} from '@vanilla-extract/css'; const a=style`style-order:2;color:red`;",
    ] {
        let result = compat(source).required("compat class literal must compile");
        assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.style_order() == Some(2))), "{source}");
        assert!(!result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if crate::style_order::reserved(style.property()))));
    }
}

#[test]
#[serial]
fn literal_metadata_when_compat_global_routes_are_aliased_uses_global_atoms() {
    for source in [
        "import {createGlobalStyle} from 'styled-components'; const Global=createGlobalStyle`body{style-order:2;color:red}`;",
        "import {globalStyle} from '@vanilla-extract/css'; globalStyle('body','style-order:2;color:red');",
        "import {Global} from '@emotion/react'; const a=<Global styles={'body{style-order:2;color:red}'} />;",
    ] {
        let result = compat(source).required("compat global literal must compile");
        assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.style_order() == Some(2))), "{source}");
    }
}

#[test]
#[serial]
fn literal_metadata_when_emotion_css_prop_has_finite_order_selects_once() {
    let source = "/** @jsxImportSource @emotion/react */\nimport {css} from '@emotion/react'; const config={get active(){trace.push('css');return true}}; const a=<div css={css`color:red;style-order:${config.active?2:3}`} />;";
    let compiled = compat(source)
        .required("Emotion literal css prop must compile")
        .code;
    let result = whole::evaluate_code(&compiled, "a.props.className");
    assert_eq!(result.trace, serde_json::json!(["css"]), "{compiled}");
    assert!(
        result
            .element
            .as_str()
            .required("Emotion literal css prop must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_metadata_when_class_names_css_is_local_keeps_the_order() {
    let source = "import {ClassNames} from '@emotion/react'; const a=<ClassNames>{({css})=><div className={css`style-order:2;color:red`} />}</ClassNames>;";
    let result = compat(source).required("ClassNames local literal css must compile");
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.style_order() == Some(2))));
}

#[test]
#[serial]
fn literal_metadata_when_frame_and_descriptor_text_is_nested_reports_no_effect() {
    for source in [
        "import {keyframes} from '@emotion/react'; const a=keyframes`from{style-order:2;opacity:0}`;",
        "import {keyframes} from 'styled-components'; const a=keyframes({from:'style-order:2;opacity:0'});",
        "import {globalCss} from '@devup-ui/react'; globalCss({fontFaces:[`font-family:test;style-order:2`]});",
    ] {
        assert!(
            compat(source)
                .required_err("compat frame or descriptor metadata must be invalid")
                .contains("has no effect"),
            "{source}"
        );
    }
}

#[test]
#[serial]
fn literal_metadata_when_numeric_tokens_are_exact_numbers_does_not_scale() {
    for value in ["+2", "2.0", "2e0", "'2'", "'\\32 '", "${2}", "${'2'}"] {
        let source = format!(
            "import {{css}} from '@devup-ui/react'; const a=css`color:red;style-order:{value}`"
        );
        let result = output(&source);
        assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.style_order() == Some(2))), "{value}");
    }
}

#[test]
#[serial]
fn literal_metadata_when_global_raw_rules_are_mixed_preserves_opaque_css() {
    let result = output(
        "import {globalCss} from '@devup-ui/react'; globalCss`@import 'a.css';@font-face{font-family:test;src:url(test.woff)}@keyframes spin{from{opacity:0}to{opacity:1}}@layer named{body{color:red;style-order:2}}`;",
    );
    let raw: String = result
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Css(style) => Some(style.css.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        raw.contains("@import") && raw.contains("@font-face") && raw.contains("@keyframes spin"),
        "{raw}"
    );
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.style_order() == Some(2) && style.layer() == Some("named"))));
}

#[test]
#[serial]
fn literal_metadata_when_invalid_text_is_multiline_locates_original_value() {
    let source = "import {css} from '@devup-ui/react';\r\nconst a=css`/* 한글 */\r\n color:red;\r\n style-order:255;`;";
    let message = error(source);
    assert!(message.starts_with("a.tsx:4:14:"), "{message}");
}

#[test]
#[serial]
fn literal_metadata_when_invalid_interpolation_branch_locates_branch() {
    let source = "import {css} from '@devup-ui/react';\nconst a=(on)=>css`style-order:${on?2:255};color:red`;";
    let message = error(source);
    let column = source
        .lines()
        .nth(1)
        .required("fixture has a second line")
        .find("255")
        .required("second line contains invalid order")
        + 1;
    assert!(
        message.starts_with(&format!("a.tsx:2:{column}:")),
        "{message}"
    );
}
