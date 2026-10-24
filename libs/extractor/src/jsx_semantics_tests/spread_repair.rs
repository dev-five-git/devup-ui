use super::*;
use crate::extract_style::ExtractStyleProperty;
use rstest::rstest;
use serial_test::serial;

#[path = "spread_boundary.rs"]
mod boundary;
#[path = "spread_grouping.rs"]
mod grouping;

fn selected(source: &str, probe: &str) -> serde_json::Value {
    let compiled = output(source);
    let actual = whole::evaluate(source, probe).element;
    let classes: Vec<_> = actual["props"]["className"]
        .as_str()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let mut declarations = Vec::new();
    css::debug::set_debug(true);
    for style in &compiled.styles {
        let (level, value) = match style {
            ExtractStyleValue::Static(style) => (style.level(), Some(style.value().to_string())),
            ExtractStyleValue::Dynamic(style) => {
                let StyleProperty::Variable { variable_name, .. } = style.extract(None) else {
                    panic!("dynamic variable");
                };
                let supplied = &actual["props"]["style"][&variable_name];
                let value = match supplied {
                    serde_json::Value::String(value)
                        if !value.is_empty() || style.property().starts_with("--") =>
                    {
                        Some(value.clone())
                    }
                    serde_json::Value::Number(value) => Some(value.to_string()),
                    _ => None,
                };
                (style.level(), value)
            }
            _ => continue,
        };
        let name = match style.extract(Some("a.tsx")) {
            Some(StyleProperty::ClassName(name)) => name,
            Some(StyleProperty::Variable { class_name, .. }) => class_name,
            None => continue,
        };
        if classes.contains(&name.as_str()) {
            declarations.push((level, value));
        }
    }
    css::debug::set_debug(false);
    declarations.sort_by_key(|(level, _)| *level);
    serde_json::Value::Array(
        [0, 1, 2, 4]
            .into_iter()
            .map(|viewport| {
                declarations
                    .iter()
                    .rfind(|(level, _)| *level <= viewport)
                    .and_then(|(_, value)| value.as_deref())
                    .map(|value| if value == "0px" { "0" } else { value })
                    .map_or(serde_json::Value::Null, Into::into)
            })
            .collect(),
    )
}

#[rstest]
#[case("color=\"red\"", "color", "red")]
#[case("p={1}", "p", "4px")]
#[case("opacity={0.3}", "opacity", ".3")]
#[serial]
fn authored_styles_when_raw_values_are_absent_remain_unchanged(
    #[case] attribute: &str,
    #[case] key: &str,
    #[case] fallback: &str,
) {
    let source = format!("{BOX}export const a=(rest)=><Box {attribute} {{...rest}}/>;");
    for input in [
        "undefined",
        "null",
        "false",
        "true",
        "NaN",
        "Infinity",
        "-Infinity",
        "''",
    ] {
        let actual = selected(&source, &format!("a({{{key}:{input}}})"));
        assert_eq!(
            actual,
            serde_json::json!([fallback, fallback, fallback, fallback]),
            "{input}"
        );
    }
    assert_eq!(
        selected(&source, "a({})"),
        serde_json::json!([fallback, fallback, fallback, fallback])
    );
}

#[rstest]
#[case("p", "1", "2")]
#[case("p", "1", "'2'")]
#[case("p", "1", "'12px'")]
#[case("p", "1", "0")]
#[case("p", "1", "-2")]
#[case("p", "1", "0.5")]
#[case("p", "1", "'.5'")]
#[case("p", "1", "'1.'")]
#[case("p", "1", "'1.e2'")]
#[case("p", "1", "'1e-2'")]
#[case("p", "1", "'+4'")]
#[case("p", "1", "'-0'")]
#[case("p", "1", "'0x10'")]
#[case("p", "1", "'1_000'")]
#[case("p", "1", "'4\\n'")]
#[case("animationDuration", "1", "2")]
#[case("animationDuration", "1", "'0.5'")]
#[case("opacity", "0.3", "2")]
#[serial]
fn raw_scalars_when_passed_through_leave_authored_units_unchanged(
    #[case] key: &str,
    #[case] fallback: &str,
    #[case] input: &str,
) {
    let source = format!("{BOX}export const a=(rest)=><Box {key}={{{fallback}}} {{...rest}}/>;");
    let actual = selected(&source, &format!("a({{{key}:{input}}})"));
    let expected = match key {
        "p" => "4px",
        "animationDuration" => "1ms",
        "opacity" => ".3",
        _ => panic!("unexpected property"),
    };
    assert_eq!(actual, serde_json::json!(vec![expected; 4]));
    assert_eq!(
        whole::evaluate(
            &source,
            &format!(
                "(()=>{{const raw={input};return Object.is(a({{{key}:raw}}).props.{key},raw);}})()"
            )
        )
        .element,
        true
    );
}

#[rstest]
#[case("p={value()}")]
#[case("p={on ? '4px' : '8px'}")]
#[case("p={on ? '4px' : null}")]
#[case("p={on ? null : '8px'}")]
#[case("p={on && '4px'}")]
#[case("p={[1,2]}")]
#[case("p='4px' {...first}")]
#[case("padding='4px'")]
#[serial]
fn authored_selection_when_raw_spread_values_vary_keeps_original_branches(#[case] attribute: &str) {
    let source = format!(
        "{BOX}export const a=(rest,on)=>{{const first={{p:'20px'}};const value=()=> '5px';return <Box {attribute} {{...rest}}/>;}};"
    );
    let key = if attribute.starts_with("padding") {
        "padding"
    } else {
        "p"
    };
    for on in [true, false] {
        for input in ["2", "null", "undefined", "false", "''"] {
            let actual = selected(&source, &format!("a({{{key}:{input}}},{on})"));
            assert_eq!(
                actual,
                selected(&source, &format!("a({{}},{on})")),
                "{attribute}/{on}/{input}"
            );
        }
    }
}

#[test]
#[serial]
fn spread_repair_effects_when_spread_getters_run_preserves_native_order() {
    let source = format!(
        "{BOX}export const a=(rest)=> <Box p={{(trace.push('value'),'5px')}} {{...rest}} id={{(trace.push('id'),'x')}}/>;"
    );
    let actual = whole::evaluate(
        &source,
        "(()=>{const s=Symbol('x');const rest=Object.create({ p: 9 });rest[s]=7;Object.defineProperty(rest,'p',{enumerable:true,get(){trace.push('get');return 2;}});const e=a(rest);return [Object.values(e.props.style),e.props[s],e.props.id];})()",
    );
    assert_eq!(actual.element, serde_json::json!([["5px"], 7, "x"]));
    assert_eq!(actual.trace, serde_json::json!(["value", "get", "id"]));
    assert_eq!(
        selected(&source, "a({ p:2 })"),
        serde_json::json!(["5px", "5px", "5px", "5px"])
    );
}

#[rstest]
#[case("p={value()}", ["5px"; 4])]
#[case("p={on ? '4px' : null}", ["4px"; 4])]
#[case("p={on ? null : '8px'}", [""; 4])]
#[case("p={[1,2]}", ["4px", "8px", "8px", "8px"])]
#[case("p='4px' {...first}", ["20px"; 4])]
#[serial]
fn authored_values_when_spread_keys_are_missing_preserve_original_levels(
    #[case] attribute: &str,
    #[case] expected: [&str; 4],
) {
    let source = format!(
        "{BOX}export const a=(rest,on)=>{{const first={{p:'20px'}};const value=()=> '5px';return <Box {attribute} {{...rest}}/>;}};"
    );
    let actual = selected(&source, "a({},true)");
    let expected = expected.map(|value| if value.is_empty() { None } else { Some(value) });
    assert_eq!(actual, serde_json::json!(expected));
}

#[test]
#[serial]
fn spread_frames_when_emitted_have_no_style_key_selection() {
    for attribute in [
        "color='red'",
        "p={1}",
        "p={value()}",
        "p={[1,2]}",
        "p={on && '4px'}",
        "p='4px' {...first}",
    ] {
        let source = format!(
            "{BOX}export const a=(rest,on)=>{{const first={{p:'20px'}};const value=()=> '5px';return <Box {attribute} {{...rest}}/>;}};"
        );
        let compiled = output(&source);
        assert!(!compiled.code.contains("__devupNumber"));
        assert!(!compiled.code.contains("-spread-"));
        for key in ["color", "p"] {
            assert!(!compiled.code.contains(&format!("\"{key}\" in ")));
        }
        assert_eq!(
            selected(&source, "a({p:2,color:'raw'},true)"),
            selected(&source, "a({},true)")
        );
    }
}
