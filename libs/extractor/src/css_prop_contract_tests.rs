use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

#[test]
fn expression_layers_keep_authored_first_use_before_styles_are_inserted() {
    use crate::{
        ExtractStyleProp,
        extract_style::{
            extract_dynamic_style::ExtractDynamicStyle, extract_layer_order::ExtractLayerOrder,
            extract_static_style::ExtractStaticStyle,
        },
        style_collector::StyleCollector,
    };
    use oxc_allocator::Allocator;
    use oxc_ast::{ast::Expression, builder::AstBuilder};
    use oxc_span::SPAN;

    // Given: an expression payload repeats a nested layer around a dynamic layer.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let first = ExtractStyleValue::Static(ExtractStaticStyle::new_with_layer(
        "color",
        "red",
        0,
        None,
        Some("zz.inner".to_string()),
    ));
    let mut dynamic = ExtractDynamicStyle::new("color", 0, "--tone", None);
    dynamic.layer = Some("aa".to_string());
    let props = [
        ExtractStyleProp::Expression {
            expression: Expression::new_identifier(SPAN, "classes", &ast),
            styles: vec![first.clone(), ExtractStyleValue::Dynamic(dynamic), first],
        },
        ExtractStyleProp::Static(ExtractStyleValue::Static(
            ExtractStaticStyle::new_with_layer("color", "blue", 0, None, Some("bb".to_string())),
        )),
    ];
    let mut collector = StyleCollector::default();

    // When: authored layer names are registered before composition inserts atoms.
    collector.use_layers(&props);

    // Then: expression payloads contribute first-use metadata, not emitted atoms.
    assert_eq!(
        collector.into_styles("test.tsx"),
        std::iter::once(ExtractStyleValue::LayerOrder(ExtractLayerOrder {
            file: "test.tsx".to_string(),
            layers: vec!["zz".into(), "zz.inner".into(), "aa".into(), "bb".into()],
        }))
        .collect()
    );
}

fn options() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    }
}

#[test]
#[serial]
fn empty_css_expression_is_rejected_when_mixed_with_valid_forms() {
    // Given: the original mixed fixture, including its invalid final attribute.
    reset_class_map();
    reset_file_map();
    let source = "export const App = ({ c }) => <><div css=\"color: red; padding: 4px\" /><div css={`color: ${c};`} /><div css /><div css={null} /><div css={undefined} /><div css={{}} /><div css={''} /><div css={} /></>;";

    // When: the public extractor applies main's strict JSX parsing contract.
    let error = extract("test.tsx", source, options())
        .err()
        .unwrap_or_else(|| panic!("invalid JSX was accepted"))
        .to_string();

    // Then: the original invalid input has the exact located parser error.
    assert_eq!(
        error,
        "test.tsx:1:192: JSX attributes must only be assigned a non-empty 'expression'."
    );
}

#[rstest]
#[case::repeated(
    "[{'@layer':{zz:{color:'red'}}},{'@layer':{aa:{color:'blue'}}},{'@layer':{zz:{color:'green'}}}]",
    vec!["zz", "aa"]
)]
#[case::conditional(
    "[flag && {'@layer':{zz:{color:'red'}}},{'@layer':{aa:{color:'blue'}}},{'@layer':{zz:{color:'green'}}}]",
    vec!["zz", "aa"]
)]
#[case::nested(
    "[{'@layer':{'zz.inner':{color:'red'}}},{'@layer':{aa:{color:'blue'}}},{'@layer':{'zz.inner':{color:'green'}}}]",
    vec!["zz", "zz.inner", "aa"]
)]
#[serial]
fn repeated_layers_keep_first_use_when_later_atoms_replace_earlier_ones(
    #[case] rules: &str,
    #[case] expected: Vec<&str>,
) {
    // Given: the first layer's first atom will lose D1 composition.
    reset_class_map();
    reset_file_map();
    let source = format!("export const view=<div css={{{rules}}} />;");

    // When: the public extractor composes all authored parts.
    let output = extract("test.tsx", &source, options()).unwrap_or_else(|error| panic!("{error}"));

    // Then: only the two winning atoms are emitted, without losing first-use metadata.
    let mut colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    colors.sort_unstable();
    assert_eq!(colors, vec![("color", "blue"), ("color", "green")]);
    let layers: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::LayerOrder(order) => Some(order.layers.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(layers, vec![expected]);
}

#[rstest]
#[case::embedded("/* not@jsxImportSource @emotion/react */\n", "", false)]
#[case::joined("/* @jsxImportSource@emotion/react */\n", "", false)]
#[case::suffix("/* @jsxImportSource @emotion/reactive */\n", "", false)]
#[case::post_header(
    "export const anchor=1;\n",
    "/* @jsxImportSource @emotion/react */\n",
    false
)]
#[case::header("/** @jsxImportSource @emotion/react */\n", "", true)]
#[case::directive("'use client';\n/*@jsxImportSource\t@emotion/react*/\n", "", true)]
#[case::import("import { jsx } from '@emotion/react';\n", "", true)]
#[case::runtime("import { jsx } from '@emotion/react/jsx-runtime';\n", "", true)]
#[serial]
fn custom_css_is_consumed_only_when_emotion_is_enabled(
    #[case] header: &str,
    #[case] later: &str,
    #[case] enabled: bool,
) {
    // Given: a custom component owns css unless an actual Emotion signal opts it in.
    reset_class_map();
    reset_file_map();
    let source = format!("{header}{later}export const view=<Custom css={{{{color:'red'}}}} />;");

    // When: the public extractor sees the file with the Emotion alias enabled.
    let output = extract("test.tsx", &source, options()).unwrap_or_else(|error| panic!("{error}"));

    // Then: only a valid header or import transfers ownership of the prop.
    assert_eq!(output.styles.len(), usize::from(enabled), "{}", output.code);
    assert_eq!(output.code.contains("css="), !enabled, "{}", output.code);
}

#[test]
#[serial]
fn layers_follow_authored_order_when_css_array_classes_are_generated() {
    // Given: layers whose first-use order differs from alphabetical order, plus a D1 override.
    reset_class_map();
    reset_file_map();
    let source = "export const view=<div css={[{color:'red','@layer':{zz:{color:'red'}}},{color:'green','@layer':{aa:{color:'blue'}}}]} />;";

    // When: public extraction composes the css array and generates its classes.
    let output = extract("test.tsx", source, options()).unwrap_or_else(|error| panic!("{error}"));

    // Then: cascade layers retain first-use order while the later unlayered color wins.
    let layers: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::LayerOrder(order) => Some(order.layers.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(layers, vec![vec!["zz".to_string(), "aa".to_string()]]);
    let plain: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.layer.is_none() => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(plain, vec![("color", "green")]);
    assert!(output.code.contains("className="), "{}", output.code);
}

#[rstest]
#[case::leading_zero("t.colors['01']", "var(--colors-01)")]
#[case::numeric_string("t.colors['1.00']", "var(--colors-1.00)")]
#[case::numeric("t.colors[1]", "var(--colors-1)")]
#[case::static_key("t.colors.primary", "var(--colors-primary)")]
#[case::template("t.colors[`01`]", "var(--colors-01)")]
#[serial]
fn theme_keys_preserve_exact_text_when_computed(#[case] read: &str, #[case] expected: &str) {
    // Given: a theme key whose spelling is part of the provider's CSS variable name.
    reset_class_map();
    reset_file_map();
    let source = format!("export const view=<div css={{t=>({{color:{read}}})}} />;");

    // When: public extraction lowers the function's theme read.
    let output = extract("test.tsx", &source, options()).unwrap_or_else(|error| panic!("{error}"));

    // Then: the generated declaration reads the exact corresponding variable.
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![("color", expected)]);
}

#[rstest]
#[case::inline(
    "",
    "[{margin:2,marginTop:0,opacity:0.5,lineHeight:1.5,'&:hover':{'padding':3}}]"
)]
#[case::named(
    "const rules=[{margin:2,marginTop:0,opacity:0.5,lineHeight:1.5,'&:hover':{'padding':3}}];",
    "rules"
)]
#[case::nested(
    "const rules=[[{margin:2,marginTop:0,opacity:0.5,lineHeight:1.5,'&:hover':{'padding':3}}]];",
    "rules"
)]
#[serial]
fn emotion_numbers_stay_pixels_when_rule_arrays_are_inlined(
    #[case] declaration: &str,
    #[case] rules: &str,
) {
    // Given: equivalent inline and constant arrays, including nested rules and unitless values.
    reset_class_map();
    reset_file_map();
    let source = format!("{declaration}export const view=<div css={{{rules}}} />;");

    // When: public extraction inlines constants and lowers the css prop.
    let output = extract("test.tsx", &source, options()).unwrap_or_else(|error| panic!("{error}"));

    // Then: Emotion lengths use px without Devup scaling; zero and unitless values stay bare.
    let mut values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    values.sort_unstable();
    assert_eq!(
        values,
        vec![
            ("line-height", "1.5"),
            ("margin", "2px"),
            ("margin-top", "0"),
            ("opacity", ".5"),
            ("padding", "3px")
        ]
    );
}
