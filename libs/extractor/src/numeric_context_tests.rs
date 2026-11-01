use std::collections::HashMap;

use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("f({ size: 4 })", serde_json::json!({"size":4}))]
#[case("f({ size: 4, nested: { size: 4 }, label: 'auto' })", serde_json::json!({"size":4,"nested":{"size":4},"label":"auto"}))]
#[case("new F({ size: 4, nested: { size: 4 }, label: 'auto' }).width", serde_json::json!({"size":4,"nested":{"size":4},"label":"auto"}))]
#[serial]
fn call_inputs_when_css_reads_result_keep_application_numbers(
    #[case] expression: &str,
    #[case] argument: serde_json::Value,
    #[values("3", "8", "0", "\"auto\"", "\"3\"")] result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{css}} from '@emotion/react';export const App=()=> <div css={{{{width:{expression}}}}}/>;"
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let output = extract("numeric.tsx", &code, option)?;
    let variables = output
        .code
        .split_once("style={")
        .ok_or("missing variables")?
        .1
        .split_once("} />")
        .ok_or("missing element end")?
        .0;
    let script = format!(
        "const seen=[];function f(input){{seen.push([typeof input.size,input]);return {result};}}function F(input){{this.width=f(input);}}const vars={variables};JSON.stringify([seen,Object.values(vars)]);"
    );
    let json = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| format!("generated variables failed: {error}"))?
        .as_string()
        .ok_or("expected JSON")?
        .to_std_string_escaped();
    // Then
    let value: serde_json::Value = serde_json::from_str(result)?;
    let expected = match value.as_f64() {
        Some(number) if number != 0.0 => serde_json::json!(format!("{number}px")),
        _ => value,
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json)?,
        serde_json::json!([[["number", argument]], [expected]])
    );
    Ok(())
}

#[rstest]
#[case(
    "@emotion/react",
    "css(css(...[{color:'red',margin:1}]),{color:'green'})",
    "1px"
)]
#[case("@emotion/react", "css(css(...pieces),{color:'green'})", "1px")]
#[case("@emotion/react", "css(css(...nested),{color:'green'})", "1px")]
#[case("@emotion/react", "css(css(...[...pieces]),{color:'green'})", "1px")]
#[case(
    "@devup-ui/react",
    "css(css(...[{color:'red',margin:1}]),{color:'green'})",
    "4px"
)]
#[case("@devup-ui/react", "css(css(...pieces),{color:'green'})", "4px")]
#[case("@devup-ui/react", "css(css(...nested),{color:'green'})", "4px")]
#[case("@devup-ui/react", "css(css(...[...pieces]),{color:'green'})", "4px")]
#[serial]
fn spread_numbers_when_composed_keep_source_library_units(
    #[case] library: &str,
    #[case] expression: &str,
    #[case] margin: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!(
        "import {{css}} from '{library}'; const pieces=[{{color:'red',margin:1}}]; const nested=[...pieces]; export const result={expression};"
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let output = extract("numeric.tsx", &code, option)?;
    // Then
    let margins: Vec<&str> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "margin" => {
                Some(style.value.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(margins, vec![margin]);
    Ok(())
}

#[rstest]
#[case(
    "return <div css={{padding:props.padding,lineHeight:props.lineHeight}}/>;",
    "props.padding"
)]
#[case(
    "const inner={padding:props.padding,lineHeight:props.lineHeight}; return <div css={inner}/>;",
    "inner[`padding`]"
)]
#[case(
    "const inner={padding:props.getPadding(),lineHeight:props.lineHeight}; return <div css={inner}/>;",
    "inner[`padding`]"
)]
#[case(
    "const inner={_hover:{padding:props.padding,lineHeight:props.lineHeight}}; return <div css={inner}/>;",
    "inner[`_hover`][`padding`]"
)]
#[serial]
fn runtime_lengths_when_css_prop_reads_values_convert_only_lengths(
    #[case] body: &str,
    #[case] read: &str,
    #[values("3", "0", "'12px'", "'3'")] input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("import {{css}} from '@emotion/react'; export function f(props){{{body}}}");
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let output = extract("numeric.tsx", &code, option)?;
    // Then
    assert!(output.code.contains("typeof"), "{}", output.code);
    assert!(output.code.contains("+`px`"), "{}", output.code);
    assert!(output.code.contains(read), "{}", output.code);
    assert_eq!(output.code.matches("typeof").count(), 1, "{}", output.code);
    let initializer = body.split_once("return").ok_or("fixture needs a return")?.0;
    let variables = output
        .code
        .split_once("style={")
        .ok_or("compiled element needs CSS variables")?
        .1
        .split_once("} />")
        .ok_or("compiled style expression needs a closing element")?
        .0;
    let script = format!(
        "let calls=0; const props={{padding:{input},lineHeight:2,getPadding(){{calls++;return this.padding;}}}}; {initializer} const vars={variables}; JSON.stringify([Object.values(vars),calls]);"
    );
    let result = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| format!("cannot evaluate generated variables: {error}"))?
        .as_string()
        .ok_or("generated variable fixture must return JSON")?
        .to_std_string_escaped();
    let (values, calls): (Vec<serde_json::Value>, usize) = serde_json::from_str(&result)?;
    let expected = match input {
        "3" => serde_json::json!("3px"),
        "0" => serde_json::json!(0),
        "'12px'" => serde_json::json!("12px"),
        "'3'" => serde_json::json!("3"),
        _ => unreachable!(),
    };
    let mut values: Vec<String> = values.iter().map(ToString::to_string).collect();
    values.sort();
    let mut expected = vec!["2".to_string(), expected.to_string()];
    expected.sort();
    assert_eq!(values, expected);
    assert_eq!(calls, usize::from(body.contains("getPadding()")));
    Ok(())
}

#[rstest]
#[case("@emotion/styled", "styled.div(inner)", "1px")]
#[case("@devup-ui/react", "styled('div',inner)", "4px")]
#[serial]
fn local_styled_numbers_when_aliased_keep_library_units(
    #[case] library: &str,
    #[case] call: &str,
    #[case] margin: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let import = if library == "@emotion/styled" {
        "import styled from '@emotion/styled';"
    } else {
        "import {styled} from '@devup-ui/react';"
    };
    let code = format!(
        "{import} export function f(){{const inner={{margin:1,p:2,mx:3,lineHeight:2,vars:{{'--raw':3}}}};const C={call};return <C/>;}}"
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([(
            "@emotion/styled".into(),
            ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..ExtractOption::default()
    };
    // When
    let output = extract("numeric.tsx", &code, option)?;
    // Then
    assert!(output.styles.iter().any(|style|matches!(style,ExtractStyleValue::Static(style) if style.property=="margin" && style.value==margin)), "{output:?}");
    assert!(output.styles.iter().any(|style|matches!(style,ExtractStyleValue::Static(style) if style.property=="padding" && style.value=="8px")), "{output:?}");
    Ok(())
}

#[test]
#[serial]
fn unknown_local_numbers_when_api_has_no_element_remain_located_error() {
    // Given
    reset_class_map();
    reset_file_map();
    let code = "import {css} from '@emotion/react'; export function f(props){const inner={padding:props.padding};return css(inner);}";
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let Err(error) = extract("numeric.tsx", code, option) else {
        panic!("CSS without an element cannot hold runtime values");
    };
    let error = error.to_string();
    // Then
    assert!(error.starts_with("numeric.tsx:1:"), "{error}");
}

#[rstest]
#[case("return <div css={{padding:3,lineHeight:2,p:3,mx:2,margin:0,vars:{'--raw':3}}}/>;")]
#[case(
    "const inner={padding:3,lineHeight:2,p:3,mx:2,margin:0,vars:{'--raw':3}}; return <div css={inner}/>;"
)]
#[serial]
fn static_numbers_when_css_prop_expands_keep_shorthands_and_unitless_values(
    #[case] body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("import {{css}} from '@emotion/react'; export function f(){{{body}}}");
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let output = extract("numeric.tsx", &code, option)?;
    // Then
    let styles: Vec<(&str, &str)> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert!(styles.contains(&("line-height", "2")), "{styles:?}");
    assert!(styles.contains(&("margin", "0")), "{styles:?}");
    assert!(styles.contains(&("--raw", "3")), "{styles:?}");
    assert!(styles.contains(&("padding", "3px")), "{styles:?}");
    assert!(styles.contains(&("margin-left", "8px")), "{styles:?}");
    assert!(!output.code.contains("typeof"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case(
    "const inner={p:props.padding,mx:props.margin,lineHeight:props.lineHeight};return <Box {...inner}/>;"
)]
#[case("return <Box p={props.padding} mx={props.margin} lineHeight={props.lineHeight}/>;")]
#[case("const inner={vars:{'--raw':props.padding}};return <div css={inner}/>;")]
#[case("return <div css={{vars:{'--raw':props.padding}}}/>;")]
#[serial]
fn bare_runtime_numbers_when_native_or_vars_are_read_keep_existing_semantics(
    #[case] body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("import {{Box}} from '@devup-ui/react';export function f(props){{{body}}}");
    let option = ExtractOption {
        import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
        ..ExtractOption::default()
    };
    // When
    let output = extract("numeric.tsx", &code, option)?;
    // Then
    assert!(!output.code.contains("typeof"), "{}", output.code);
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_))),
        "{output:?}"
    );
    Ok(())
}
