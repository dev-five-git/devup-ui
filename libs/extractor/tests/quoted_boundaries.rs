use std::{collections::HashMap, error::Error};

use extractor::{ExtractOption, extract, extract_style::extract_style_value::ExtractStyleValue};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(r"content: 'a;${p => p.label}';")]
#[case(r"content: '${p => p.label}{';")]
#[case(r"content: 'a}${p => p.label}';")]
#[case(r"content: 'a:${p => p.label}';")]
#[case(r"content: 'a;${p => p.first}${p => p.last}{';")]
#[case(r"content: 'a\'${p => p.label}{';")]
#[serial]
fn interpolation_in_a_quoted_value_stays_a_value(#[case] css: &str) -> Result<(), Box<dyn Error>> {
    let _state = extractor::test_state::TestStateGuard::new();
    let source =
        format!("import {{styled}} from '@devup-ui/react';export const C=styled('div')`{css}`;");
    let output = extract("quoted-boundaries.tsx", &source, options())?;
    assert!(output.styles.iter().any(
        |style| matches!(style, ExtractStyleValue::Dynamic(style) if style.property() == "content")
    ));
    Ok(())
}

#[rstest]
#[case(r#"&[title="a,  >  b"]"#)]
#[case(r#"&[title="a;{b}"]"#)]
#[case(r#"&[title="/* keep */"]"#)]
#[serial]
fn template_selector_preserves_quoted_bytes(#[case] selector: &str) -> Result<(), Box<dyn Error>> {
    let _state = extractor::test_state::TestStateGuard::new();
    let source = format!(
        "import {{css}} from '@devup-ui/react';export const cls=css`{selector}{{color:red}}`;"
    );
    let output = extract("quoted-boundaries.tsx", &source, options())?;
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.selector.as_ref().is_some_and(|actual| actual.to_string() == selector))), "{:?}", output.styles);
    Ok(())
}

fn options() -> ExtractOption {
    ExtractOption {
        package: "@devup-ui/react".to_string(),
        css_dir: "@devup-ui/react".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::new(),
    }
}

#[rstest]
#[case(concat!(r#"css`[title="&"]"#, "{", "color:red", "}`"), r#"& [title="&"]"#)]
#[case(r#"css({selectors:{'[title="&"]':{color:'red'}}})"#, r#"&[title="&"]"#)]
#[serial]
fn bare_selectors_keep_their_anchor_when_ampersands_are_quoted(
    #[case] expression: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn Error>> {
    // Given template descendant or object self-selector syntax.
    let _state = extractor::test_state::TestStateGuard::new();
    let source = format!("import {{css}} from '@devup-ui/react';export const cls={expression};");
    // When the style is extracted.
    let output = extract("quoted-anchors.tsx", &source, options())?;
    // Then the literal ampersand cannot stand in for the element's anchor.
    let selectors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => style.selector.as_ref().map(ToString::to_string),
            _ => None,
        })
        .collect();
    assert_eq!(selectors, vec![expected]);
    Ok(())
}

#[rstest]
#[case(r"focus:[&[title='a&b']]:p-4", r"&:focus[title='a&b']")]
#[case(r"[&[title='a&b']]:focus:p-4", r"&[title='a&b']:focus")]
#[serial]
fn tailwind_composition_keeps_ampersands_when_they_are_quoted(
    #[case] class: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn Error>> {
    // Given a stacked arbitrary selector variant.
    let _state = extractor::test_state::TestStateGuard::new();
    let source = format!(
        "import {{Box}} from '@devup-ui/react';export const C=()=> <Box className=\"{class}\"/>;"
    );
    // When the utility is extracted.
    let output = extract("quoted-tailwind.tsx", &source, options())?;
    // Then substitution preserves the attribute's literal payload.
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.selector.as_ref().is_some_and(|selector| selector.to_string() == expected)
    )), "{:?}", output.styles);
    Ok(())
}

#[test]
#[serial]
fn global_pseudo_nesting_preserves_quoted_ampersands() -> Result<(), Box<dyn Error>> {
    let _state = extractor::test_state::TestStateGuard::new();
    let source = concat!(
        "import{globalCss}from'@devup-ui/react';globalCss(",
        "{_hover:{selectors:{",
        r#""&[title='&']": {color:'red'}"#,
        "}}});"
    );
    let output = extract("quoted-global.tsx", source, options())?;
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.selector.as_ref().is_some_and(|selector| selector.to_string() == "*:hover[title='&']")
    )), "{:?}", output.styles);
    Ok(())
}
