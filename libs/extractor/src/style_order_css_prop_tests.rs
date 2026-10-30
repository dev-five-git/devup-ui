use std::{collections::HashMap, error::Error};

use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

use crate::{
    ExtractOption, ImportAlias, extract, extract_style::extract_style_value::ExtractStyleValue,
};

const FIXTURES: &str = include_str!("../../../test-fixtures/style-order.json");

fn emotion_option() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([
            ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
            (
                "@emotion/react/jsx-runtime".to_string(),
                ImportAlias::NamedToNamed,
            ),
        ]),
        ..ExtractOption::default()
    }
}

fn source_for(runtime: bool, value: &str) -> String {
    if runtime {
        format!(
            "import {{css}} from '@emotion/react';\nimport {{jsx as render}} from '@emotion/react/jsx-runtime';\nexport const App=()=> render('div', {{css: {value}}});"
        )
    } else {
        format!(
            "import {{css}} from '@emotion/react';\nexport const App=()=> <div css={{{value}}}/>;"
        )
    }
}

fn location(source: &str, leaf: &str) -> Result<String, Box<dyn Error>> {
    let offset = source.rfind(leaf).ok_or("missing offending value")?;
    let before = source.get(..offset).ok_or("invalid fixture offset")?;
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .ok_or("missing line")?
        .chars()
        .count()
        + 1;
    Ok(format!("css-order.tsx:{line}:{column}:"))
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn css_prop_rejects_original_quoted_value_when_css_call_is_consumed(
    #[case] runtime: bool,
) -> Result<(), Box<dyn Error>> {
    // Given
    let source = if runtime {
        source_for(true, "css({color:'red',styleOrder:'100px'})")
    } else {
        "import {css} from '@emotion/react'; export const App=()=> <div css={css({color:'red',styleOrder:'100px'})}/>".to_string()
    };
    reset_class_map();
    reset_file_map();
    let expected = location(&source, "'100px'")?;
    // When
    let error = extract("css-order.tsx", &source, emotion_option())
        .err()
        .ok_or("malformed order unexpectedly compiled")?
        .to_string();
    // Then
    assert!(
        error.lines().any(|line| line.starts_with(&expected)),
        "{error}"
    );
    assert!(error.contains("`'100px'`"), "{error}");
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn css_prop_reports_original_branch_when_nested_composition_is_consumed(
    #[case] runtime: bool,
) -> Result<(), Box<dyn Error>> {
    // Given
    let source = source_for(
        runtime,
        "css({margin:0}, [css({color:'red',styleOrder:on ? 2 : '\\u0030\\u0031'})])",
    );
    reset_class_map();
    reset_file_map();
    let expected = location(&source, "'\\u0030\\u0031'")?;
    // When
    let error = extract("css-order.tsx", &source, emotion_option())
        .err()
        .ok_or("malformed branch unexpectedly compiled")?
        .to_string();
    // Then
    assert!(
        error.lines().any(|line| line.starts_with(&expected)),
        "{error}"
    );
    assert!(error.contains("`'\\u0030\\u0031'`"), "{error}");
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn css_prop_follows_shared_fixtures_when_rules_are_consumed(
    #[case] runtime: bool,
    #[values(
        "RULES",
        "css(RULES)",
        "css({margin:0}, [css(RULES)])",
        "[{margin:0}, [css(RULES)]]"
    )]
    shape: &str,
) -> Result<(), Box<dyn Error>> {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES)?;
    for fixture in fixtures.as_array().ok_or("expected fixture array")? {
        // Given
        let expression = fixture["expression"].as_str().ok_or("missing expression")?;
        let valid = fixture["valid"].as_bool().ok_or("missing validity")?;
        let rules = format!("{{color:'red',styleOrder:{expression}}}");
        let source = source_for(runtime, &shape.replace("RULES", &rules));
        reset_class_map();
        reset_file_map();
        // When
        let result = extract("css-order.tsx", &source, emotion_option());
        // Then
        if valid {
            result.map_err(|error| format!("{source}\n{error}"))?;
        } else {
            let error = result
                .err()
                .ok_or_else(|| format!("invalid order compiled: {source}"))?
                .to_string();
            let leaf = expression
                .rsplit_once(" : ")
                .or_else(|| expression.rsplit_once(" && "))
                .map_or(expression, |(_, leaf)| leaf);
            let expected = location(&source, leaf)?;
            assert!(
                error.lines().any(|line| line.starts_with(&expected)),
                "{source}\n{error}"
            );
            assert!(error.contains("`styleOrder()` cannot use"), "{error}");
        }
    }
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn css_prop_selects_expected_layer_when_static_order_is_valid(
    #[case] runtime: bool,
    #[values(
        "RULES",
        "css(RULES)",
        "css({margin:0}, [css(RULES)])",
        "[{margin:0}, [css(RULES)]]"
    )]
    shape: &str,
) -> Result<(), Box<dyn Error>> {
    for (expression, expected) in [
        ("1", 1),
        ("254", 254),
        ("100.0", 100),
        ("1e2", 100),
        ("'100'", 100),
        ("`100`", 100),
        ("+'01'", 1),
        ("+'0x10'", 16),
        ("+'1e2'", 100),
        ("+'1.0'", 1),
        ("+true", 1),
        ("-'-1'", 1),
    ] {
        // Given
        let rules = format!("{{color:'red',styleOrder:{expression}}}");
        let source = source_for(runtime, &shape.replace("RULES", &rules));
        reset_class_map();
        reset_file_map();
        // When
        let output = extract("css-order.tsx", &source, emotion_option())?;
        // Then
        let orders: Vec<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Static(style) if style.property == "color" => {
                    Some(style.style_order)
                }
                _ => None,
            })
            .collect();
        assert_eq!(orders, vec![Some(expected)], "{source}");
    }
    Ok(())
}
