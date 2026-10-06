use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, css_payload, extract, has_static};

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn exact_declarator_compiles_when_its_sibling_requires_the_browser(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {createTheme,style} from '@vanilla-extract/css';\nconst tokens={space:'8px'}, browser=window.document;\nexport const [theme,vars]=createTheme(tokens);\nexport const box=style({margin:vars.space}), runtime=window.innerWidth;\ndocument.title='kept';\nthrow new Error('runtime only');";
    // When
    let output = extract(extension, source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    assert!(has_static(&output, "--space-0-1", "8px"));
    assert!(output.code.contains("theme-0-0"));
    assert_preserved(
        &output.code,
        "const browser=window.document;export const runtime=window.innerWidth;document.title='kept';throw new Error('runtime only');",
    );
    Ok(())
}

#[rstest]
#[case("tsx")]
#[case("jsx")]
#[serial]
fn jsx_and_handlers_remain_when_a_theme_tuple_is_selected(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {createTheme} from '@vanilla-extract/css';\nexport const [theme,vars]=createTheme({space:'8px'});\nconst onClick=()=>document.body.append(window.location.href);\nexport const view=<button className={theme} onClick={onClick}>{window.name}</button>;";
    // When
    let output = extract(extension, source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "--space-0-1", "8px"));
    assert!(output.code.contains("var(--space-0-1)"));
    assert!(output.code.contains("theme-0-0"));
    assert_preserved(
        &output.code,
        "const onClick=()=>document.body.append(window.location.href);export const view=<button className={theme} onClick={onClick}>{window.name}</button>;",
    );
    Ok(())
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn helper_allocates_twice_when_two_exact_initializers_call_it(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let styling = "import {createVar,fontFace,createContainer,layer,style,keyframes,createThemeContract,createTheme} from '@vanilla-extract/css';\ncreateVar();fontFace({src:'local(Inter)'});createContainer();layer();\nstyle({opacity:0.5});keyframes({to:{opacity:1}});\nfunction make(space){const contract=createThemeContract({space:null});const theme=createTheme(contract,{space});return {theme,token:contract.space,box:style({margin:contract.space})};}\nexport const first=make('8px');\nexport const second=make('12px');\nexport const tail=createVar();";
    let source =
        format!("{styling}\nconst unrelated=window.document;\nthrow new Error('runtime only');");
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    for (property, value) in [
        ("margin", "var(--space-0-4)"),
        ("margin", "var(--space-0-6)"),
        ("--space-0-4", "8px"),
        ("--space-0-6", "12px"),
    ] {
        assert!(has_static(&output, property, value), "{property}:{value}");
    }
    for identifier in ["theme-0-5", "theme-0-7", "var(--var-0-8)", "f0_2", "f0_3"] {
        assert!(output.code.contains(identifier), "{}", output.code);
    }
    assert!(!output.code.contains("make('8px')"));
    assert!(!output.code.contains("make('12px')"));
    assert_preserved(
        &output.code,
        "const unrelated=window.document;throw new Error('runtime only');",
    );
    Ok(())
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn css_matches_stylesheet_when_only_unrelated_runtime_code_is_added(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let styling = "import {createTheme,style,globalStyle} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const box=style({padding:8,margin:vars.space});globalStyle('body',{margin:0});";
    let control = extract("css.ts", styling)?;
    assert!(has_static(&control, "padding", "8px"));
    assert!(has_static(&control, "margin", "var(--space-0-1)"));
    let mixed =
        format!("{styling}\nconst browser=window.document;throw new Error('not build time');");
    // When
    let output = extract(extension, &mixed)?;
    // Then
    assert_eq!(css_payload(&output), css_payload(&control));
    assert_consumed(&output);
    assert_preserved(
        &output.code,
        "const browser=window.document;throw new Error('not build time');",
    );
    Ok(())
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn required_host_input_errors_at_its_original_site_when_theme_tokens_are_unknown(
    #[case] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {createTheme} from '@vanilla-extract/css';\nconst unrelated=window.document;\nexport const [theme,vars]=createTheme({space:window.innerWidth});";
    // When
    let error = extract(extension, source)
        .err()
        .ok_or("unknown required theme input survived extraction")?
        .to_string();
    // Then
    assert!(
        error.contains(&format!("/mixed.{extension}:3:46:")),
        "{error}"
    );
    assert!(error.contains("window.innerWidth"), "{error}");
    assert!(
        error.contains("build time") || error.contains("build-time"),
        "{error}"
    );
    assert!(error.contains("Fix:"), "{error}");
    assert!(
        error.contains("static") || error.contains("exact") || error.contains("known"),
        "{error}"
    );
    Ok(())
}
