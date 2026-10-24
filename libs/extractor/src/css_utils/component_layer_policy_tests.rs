use crate::{ExtractOption, ImportAlias, extract};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[rstest]
#[case("base.reset", "base.reset")]
#[case("한글.é", "한글.é")]
#[case("-foo.--bar", "-foo.--bar")]
#[case(r"\62 ase", "base")]
#[case(r"\31 23", r"\31 23")]
#[case(r"a\.b", r"a\.b")]
#[case(r"a\ b", r"a\ b")]
#[case(r"\-", r"\-")]
#[case(r"\0 ", "�")]
#[case(r"\110000 ", "�")]
#[case(" base/**/.reset ", "base.reset")]
#[case("default", "default")]
#[case("framework.default", "framework.default")]
#[case(r"base.\72 eset", "base.reset")]
#[case(r"\62 ase.reset", "base.reset")]
#[test]
fn names_are_canonical_when_css_identifiers_use_unicode_or_escapes(
    #[case] source: &str,
    #[case] expected: &str,
) {
    assert_eq!(super::parse_layer_name(source).as_deref(), Some(expected));
}

#[rstest]
#[case("")]
#[case(" ")]
#[case("-")]
#[case("1base")]
#[case("base..reset")]
#[case("base.")]
#[case("base reset")]
#[case("base,reset")]
#[case("foo@bar")]
#[case("inherit")]
#[case("INITIAL")]
#[case("base.unset")]
#[case("ReVeRt-LaYeR")]
#[case("base . reset")]
#[case("base. reset")]
#[case("base .reset")]
#[case("base\t.reset")]
#[case("base.\nreset")]
#[case("base /**/ .reset")]
#[case("base./**/ reset")]
#[case(r"\69 nitial")]
#[case("a\\")]
#[case("a\\\n")]
#[test]
fn names_are_rejected_when_layer_identifiers_are_invalid_or_reserved(#[case] source: &str) {
    assert_eq!(super::parse_layer_name(source), None);
}

fn component_source(api: &str, styles: &str) -> String {
    let imports = "import { css, styled } from '@devup-ui/react';\nimport { ClassNames } from '@emotion/react';\n";
    let css = if styles.starts_with('`') {
        format!("css{styles}")
    } else {
        format!("css({styles})")
    };
    let expression = match api {
        "css" => css,
        "styled" if styles.starts_with('`') => format!("styled.div{styles}"),
        "styled" => format!("styled.div({styles})"),
        "prop" => format!("<div css={{{styles}}} />"),
        "classnames" => {
            format!("<ClassNames>{{({{css}}) => <div className={{{css}}} />}}</ClassNames>")
        }
        _ => panic!("unsupported fixture API"),
    };
    format!("{imports}export const result = {expression};")
}

#[rstest]
#[case("")]
#[case("inherit")]
#[case("base..reset")]
#[case("123")]
#[case("base .reset")]
#[case("base. reset")]
#[test]
#[serial]
fn object_names_fail_at_their_keys_when_component_layers_are_invalid(
    #[values("css", "styled", "prop", "classnames")] api: &str,
    #[case] name: &str,
) {
    let key = serde_json::to_string(name).unwrap_or_else(|error| panic!("{error}"));
    let source = component_source(
        api,
        &format!("{{ '@layer': {{ {key}: {{ color: 'red' }} }} }}"),
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let error = extract("layer.tsx", &source, option)
        .err()
        .unwrap_or_else(|| panic!("expected invalid-layer error: {source}"))
        .to_string();
    assert!(error.starts_with("layer.tsx:3:"), "{error}");
    assert!(
        error.contains("name the layer") && error.contains("globalCss"),
        "{error}"
    );
}

#[rstest]
#[case("")]
#[case("initial")]
#[case("base..reset")]
#[case("base,theme")]
#[case("base .reset")]
#[case("base. reset")]
#[test]
#[serial]
fn text_names_fail_at_their_blocks_when_component_layers_are_invalid(
    #[values("css", "styled", "prop", "classnames")] api: &str,
    #[case] name: &str,
) {
    let source = component_source(api, &format!("`@layer {name} {{ color: red; }}`"));
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let error = extract("layer.tsx", &source, option)
        .err()
        .unwrap_or_else(|| panic!("expected invalid-layer error: {source}"))
        .to_string();
    assert!(error.starts_with("layer.tsx:3:"), "{error}");
    assert!(
        error.contains("name the layer") && error.contains("globalCss"),
        "{error}"
    );
}

#[rstest]
#[case(r"@layer a\{b { color: red; }", r"a\{b")]
#[case(r"@layer/**/base { color: red; }", "base")]
#[case(r"/* before */ @layer base/**/.reset { color: red; }", "base.reset")]
#[test]
fn escaped_layer_names_keep_rules_when_escapes_contain_css_delimiters(
    #[case] text: &str,
    #[case] layer: &str,
) {
    let styles = super::css_to_style(text, 0, &None);
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.layer.as_deref(), style.value.as_str()))
            .collect::<Vec<_>>(),
        vec![(Some(layer), "red")]
    );
}

#[test]
#[serial]
fn anonymous_blocks_fail_when_the_layer_keyword_touches_the_brace() {
    let source =
        "import { css } from '@devup-ui/react';\nexport const result = css`@layer{color:red;}`;";
    let error = extract("layer.tsx", source, ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("expected anonymous-layer error"))
        .to_string();
    assert!(error.starts_with("layer.tsx:2:27:"), "{error}");
    assert!(
        error.contains("name the layer") && error.contains("globalCss"),
        "{error}"
    );
}

#[test]
#[serial]
fn equivalent_names_compose_when_text_escapes_match_object_identifiers() {
    let source = r"import { css } from '@devup-ui/react';
export const result = css({ '@layer': { base: { color: 'red' } } }, css`@layer \62 ase { color: blue; }`);";
    let output = extract(
        "layer.tsx",
        source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => {
                Some((style.layer.as_deref(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![(Some("base"), "blue")]);
}

#[test]
#[serial]
fn global_layers_stay_accepted_when_they_are_anonymous_or_declare_order() {
    let source = "import { globalCss } from '@devup-ui/react';\nglobalCss`@layer base, theme; @layer { body { color: red; } }`;";
    let output = extract("layer.tsx", source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Css(style) if style.css.contains("@layer") && style.css.contains("red"))));
}

#[rstest]
#[case("`@LAYER default { color: red; }`", "default")]
#[case("`@LaYeR framework.default { color: red; }`", "framework.default")]
#[case("`@LAYER Base { color: red; }`", "Base")]
#[case("{ '@layer': { default: { color: 'red' } } }", "default")]
#[case(
    "{ '@layer': { 'framework.default': { color: 'red' } } }",
    "framework.default"
)]
#[test]
#[serial]
fn default_layers_keep_metadata_when_component_apis_read_text_or_objects(
    #[values("css", "styled", "prop", "classnames")] api: &str,
    #[case] styles: &str,
    #[case] layer: &str,
) {
    let source = component_source(api, styles);
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let output = extract("layer.tsx", &source, option).unwrap_or_else(|error| panic!("{error}"));
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => {
                Some((style.layer.as_deref(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![(Some(layer), "red")]);
}

#[rstest]
#[test]
#[serial]
fn uppercase_layer_order_fails_when_component_apis_read_statements(
    #[values("css", "styled", "prop", "classnames")] api: &str,
) {
    let source = component_source(api, "`@LAYER base, theme;`");
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".to_string(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    let error = extract("layer.tsx", &source, option)
        .err()
        .unwrap_or_else(|| panic!("expected layer-order error: {source}"))
        .to_string();
    assert!(error.starts_with("layer.tsx:3:"), "{error}");
    assert!(
        error.contains("@LAYER base, theme;") && error.contains("globalCss"),
        "{error}"
    );
}
