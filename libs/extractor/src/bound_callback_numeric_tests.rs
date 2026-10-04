use super::*;
use crate::extract_style::extract_static_style::ExtractStaticStyle;

fn option() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([
            (
                "@emotion/styled".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
            (
                "styled-components".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
        ]),
        ..ExtractOption::default()
    }
}

#[rstest]
#[case(("import styled from '@emotion/styled';", "3px"))]
#[case(("import styled from 'styled-components';", "3px"))]
#[case(("import {styled} from '@devup-ui/react';", "12px"))]
#[serial]
fn literal_units_match_inline_when_callbacks_are_bound(
    #[case] library: (&str, &str),
    #[values(false, true)] bound: bool,
    #[values(false, true)] function: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let (import, padding) = library;
    let rules = "{padding:3,p:4,lineHeight:2,margin:0,vars:{'--raw':5},_hover:{padding:3},'@layer':{base:{padding:3}}}";
    let callback = if function {
        format!("function(p) {{ return {rules}; }}")
    } else {
        format!("p => ({rules})")
    };
    let site = if bound {
        format!("const rules = {callback}; export const Choice = styled.div(rules);")
    } else {
        format!("export const Choice = styled.div({callback});")
    };
    // When
    let output = extract("callback-units.tsx", &format!("{import} {site}"), option())?;
    // Then
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
    assert_eq!(
        values
            .iter()
            .filter(|(key, value)| *key == "padding" && *value == padding)
            .count(),
        3,
        "{output:?}"
    );
    assert!(values.contains(&("padding", "16px")), "{output:?}");
    assert!(values.contains(&("line-height", "2")), "{output:?}");
    assert!(values.contains(&("margin", "0")), "{output:?}");
    assert!(values.contains(&("--raw", "5")), "{output:?}");
    Ok(())
}

#[rstest]
#[case(("import styled from '@emotion/styled';", "3px", "4px"))]
#[case(("import styled from 'styled-components';", "3px", "4px"))]
#[case(("import {styled} from '@devup-ui/react';", "12px", "16px"))]
#[serial]
fn selected_numeric_rules_when_callbacks_short_circuit_keep_library_units(
    #[case] library: (&str, &str, &str),
    #[values("||", "??", "&&", "condition")] choice: &str,
    #[values(false, true)] bound: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let (import, left_padding, right_padding) = library;
    let left = "({padding:3,lineHeight:2,margin:0,vars:{'--raw':5}})";
    let right = "{padding:4,lineHeight:6,margin:0,vars:{'--raw':9}}";
    let rules = if choice == "condition" {
        format!("p.input.padding === 3 && {right}")
    } else {
        format!("{left} {choice} {right}")
    };
    let callback = format!("p => ({rules})");
    let site = if bound {
        format!("const rules={callback};export const Choice=styled.div(rules);")
    } else {
        format!("export const Choice=styled.div({callback});")
    };
    // When
    let output = extract(
        "callback-logical-units.tsx",
        &format!("{import}{site}"),
        ExtractOption {
            single_css: true,
            ..option()
        },
    )?;
    // Then: evaluate the generated class selection, not just the emitted atoms.
    let classes = output
        .code
        .split_once("className={")
        .ok_or("generated className")?
        .1
        .split_once("} style=")
        .ok_or("generated style attribute")?
        .0;
    let atoms = output
        .styles
        .iter()
        .map(|style| {
            style
                .extract(None)
                .ok_or("logical numeric atom has a class identity")
                .map(|class| (class.to_string(), style))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for input in [3, 4] {
        let script =
            format!("const rest={{input:{{padding:{input}}}}};const className='';({classes});");
        let mut context = boa_engine::Context::default();
        let names = context
            .eval(boa_engine::Source::from_bytes(&script))?
            .to_string(&mut context)?
            .to_std_string_escaped();
        let mut selected: Vec<_> = atoms
            .iter()
            .filter(|(class, _)| names.split_whitespace().any(|name| name == class))
            .map(|(_, style)| (*style).clone())
            .collect();
        selected.sort_unstable();
        let declarations = if choice == "condition" && input == 4 {
            vec![]
        } else if choice == "&&" || choice == "condition" {
            vec![
                ("--raw", "9"),
                ("line-height", "6"),
                ("margin", "0"),
                ("padding", right_padding),
            ]
        } else {
            vec![
                ("--raw", "5"),
                ("line-height", "2"),
                ("margin", "0"),
                ("padding", left_padding),
            ]
        };
        let mut expected: Vec<_> = declarations
            .into_iter()
            .map(|(property, value)| {
                ExtractStyleValue::Static(ExtractStaticStyle::new(property, value, 0, None))
            })
            .collect();
        expected.sort_unstable();
        assert_eq!(
            selected, expected,
            "{choice}, input={input}: {}",
            output.code
        );
    }
    Ok(())
}

#[rstest]
#[case(("native.div(rules)", "emotion.div(rules)"))]
#[case(("emotion.div(rules)", "native.div(rules)"))]
#[serial]
fn use_site_units_stay_independent_when_one_callback_serves_both_libraries(
    #[case] sites: (&str, &str),
    #[values("p => ({ padding: 3 })", "function(p) {return { padding: 3 };}")] callback: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let (first, last) = sites;
    let source = format!(
        "import emotion from '@emotion/styled'; import {{styled as native}} from '@devup-ui/react'; const rules={callback}; export const First={first}; export const Last={last};"
    );
    // When
    let output = extract("callback-mixed.tsx", &source, option())?;
    // Then
    let mut paddings: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "padding" => {
                Some(style.value.as_str())
            }
            _ => None,
        })
        .collect();
    paddings.sort_unstable();
    assert_eq!(paddings, vec!["12px", "3px"], "{output:?}");
    Ok(())
}

#[rstest]
#[case("import styled from '@emotion/styled';")]
#[case("import {styled} from '@devup-ui/react';")]
#[serial]
fn runtime_values_and_call_inputs_stay_ordinary_when_callback_is_bound(
    #[case] import: &str,
    #[values("3", "0", "'3'", "'auto'")] input: &str,
    #[values(false, true)] bound: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let callback = "({lineHeight,raw,input={ size: 3, nested: { size: 4 } }}) => ({padding:consume(input,{ size: 3, nested: { size: 4 } }),lineHeight,vars:{'--raw':raw}})";
    let site = if bound {
        format!("const rules={callback};export const Choice=styled.div(rules);")
    } else {
        format!("export const Choice=styled.div({callback});")
    };
    // When
    let output = extract("callback-values.tsx", &format!("{import}{site}"), option())?;
    // Then
    let variables = output
        .code
        .split_once("style={")
        .ok_or("missing style")?
        .1
        .split_once("} />")
        .ok_or("missing JSX end")?
        .0;
    let script = format!(
        "const seen=[]; const rest={{lineHeight:2,raw:7}};const style={{}};function consume(data,explicit){{seen.push(data,explicit);return {input};}}const variables={variables};JSON.stringify([seen,Object.values(variables)]);"
    );
    let json = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| format!("generated callback failed: {error}\n{script}"))?
        .as_string()
        .ok_or("expected JSON")?
        .to_std_string_escaped();
    let padding = match input {
        "3" => serde_json::json!("3px"),
        "0" => serde_json::json!(0),
        "'3'" => serde_json::json!("3"),
        "'auto'" => serde_json::json!("auto"),
        _ => unreachable!(),
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json)?,
        serde_json::json!([[{"size":3,"nested":{"size":4}},{"size":3,"nested":{"size":4}}], [padding,2,7]])
    );
    Ok(())
}
