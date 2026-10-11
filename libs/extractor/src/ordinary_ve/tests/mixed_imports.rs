use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, has_static};
use crate::{ExtractStyleValue, ResolvedModule, extract_with_modules};

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn imported_styles_keep_d1_and_selector_identity_when_the_entry_is_mixed(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let resolver = |source: &str, _: &str| {
        (source == "./theme.css").then(|| ResolvedModule {
            path: "/theme.css.ts".into(),
            code: "import {createTheme,style} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const base=style({color:'red',padding:8,selectors:{'&:hover':{color:'orange'}}});".into(),
        })
    };
    let styling = "import {style,globalStyle} from '@vanilla-extract/css';import {base,vars} from './theme.css';export const button=style([base,{color:'blue',margin:vars.space,selectors:{'&:hover':{color:'green'}}}]);globalStyle(`${base}:focus`,{outlineColor:'purple'});";
    let source = format!(
        "{styling}\nconst handler=()=>document.body.append(window.name), browser=window.document;throw new Error('runtime only');"
    );
    // When
    let output = extract_with_modules(
        &format!("/mixed.{extension}"),
        &source,
        super::option(),
        false,
        &resolver,
    )?;
    // Then
    assert_consumed(&output);
    for (property, value) in [
        ("color", "blue"),
        ("color", "green"),
        ("padding", "8px"),
        ("margin", "var(--space-0-1)"),
    ] {
        assert!(has_static(&output, property, value), "{property}:{value}");
    }
    for losing in ["red", "orange"] {
        assert!(!has_static(&output, "color", losing));
    }
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style)
        if style.property == "outline-color" && style.value == "purple"
        && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == ".f0_0:focus"))));
    assert!(output.code.contains("f0_0"));
    assert!(output.code.contains("f1_0"));
    assert!(
        output.code.contains("./theme.css"),
        "producer edge removed: {}",
        output.code
    );
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/theme.css.ts")
    );
    assert_preserved(
        &output.code,
        "const handler=()=>document.body.append(window.name);const browser=window.document;throw new Error('runtime only');",
    );
    Ok(())
}
