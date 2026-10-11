use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract, has_static};
use crate::ExtractStyleValue;

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn all_registered_roots_compile_when_only_the_entry_contains_runtime_code(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let styling = "import {style,globalStyle,styleVariants,keyframes,fontFace,globalFontFace,createVar,fallbackVar,createContainer,layer,globalLayer,createThemeContract,createGlobalThemeContract,assignVars,createTheme,createGlobalTheme} from '@vanilla-extract/css';\nexport const token=createVar({syntax:'<length>',inherits:false,initialValue:'0px'},'size');\nexport const fallback=fallbackVar(token,'4px');\nexport const container=createContainer('panel');\nexport const localLayer=layer('base');\nglobalLayer('reset');\nconst contract=createThemeContract({colors:{bg:null}});\nconst globalContract=createGlobalThemeContract({accent:'app-accent'});\nexport const dark=createTheme(contract,{colors:{bg:'black'}});\nexport const [light,vars]=createTheme({space:'8px'});\nexport const globals=createGlobalTheme(':root',{gap:'2px'});\nexport const nothing=createGlobalTheme('.app',globalContract,{accent:'purple'});\nexport const family=fontFace({src:'local(Inter)'});\nglobalFontFace('Icons',{src:'local(Icons)'});\nconst motion=keyframes({from:{opacity:0},to:{opacity:1}});\nconst assigned=assignVars(contract,{colors:{bg:'white'}});\nexport const variants=styleVariants({small:4},value=>({padding:value}));\nexport const tones=styleVariants({quiet:{opacity:0.5}});\nexport const box=style({vars:assigned,background:contract.colors.bg,margin:vars.space,width:fallback,containerName:container,fontFamily:family,animationName:motion});\nglobalStyle('body',{color:'navy'});";
    let source =
        format!("{styling}\nconst browser=window.document;\nthrow new Error('runtime only');");
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    for (property, value) in [
        ("background", "var(--colors-bg-0-3)"),
        ("--colors-bg-0-3", "black"),
        ("--colors-bg-0-3", "white"),
        ("--space-0-6", "8px"),
        ("--gap-0-7", "2px"),
        ("--app-accent", "purple"),
        ("width", "var(--size-0-0,4px)"),
        ("container-name", "panel-0-1"),
        ("padding", "4px"),
        ("color", "navy"),
    ] {
        assert!(has_static(&output, property, value), "{property}:{value}");
    }
    for identifier in ["theme-0-4", "theme-0-5", "font-0-8", "var(--gap-0-7)"] {
        assert!(output.code.contains(identifier), "{}", output.code);
    }
    assert!(!output.code.contains("createGlobalTheme("));
    assert!(output.code.contains("undefined"), "{}", output.code);
    assert_preserved(
        &output.code,
        "export const fallback='var(--size-0-0, 4px)';",
    );
    assert_preserved(&output.code, "export const nothing=undefined;");
    assert_preserved(
        &output.code,
        "const browser=window.document;throw new Error('runtime only');",
    );
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Keyframes(frames) if frames.keyframes.contains_key("from") && frames.keyframes.contains_key("to"))));
    for family in ["font-0-8", "Icons"] {
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::FontFace(font) if font.properties.values().any(|value| value.contains(family)))));
    }
    for effect in ["@property --size-0-0", "base-0-2", "reset"] {
        assert!(
            output.styles.iter().any(
                |style| matches!(style, ExtractStyleValue::Css(css) if css.css.contains(effect))
            ),
            "{effect}"
        );
    }
    Ok(())
}

#[rstest]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn pure_stylesheet_captures_fallback_and_void_when_native_results_are_exported(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {createVar,fallbackVar,style,createGlobalThemeContract,createGlobalTheme} from '@vanilla-extract/css';const token=createVar('size');export const fallback=fallbackVar(token,'4px');const contract=createGlobalThemeContract({accent:'app-accent'});export const nothing=createGlobalTheme('.app',contract,{accent:'purple'});export const box=style({width:fallback});";
    // When
    let output = extract(extension, source)?;
    // Then
    assert!(has_static(&output, "width", "var(--size-0-0,4px)"));
    assert!(has_static(&output, "--app-accent", "purple"));
    assert_preserved(
        &output.code,
        "export const fallback='var(--size-0-0, 4px)';",
    );
    assert_preserved(&output.code, "export const nothing=undefined;");
    assert_consumed(&output);
    Ok(())
}
