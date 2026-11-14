use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box {...rest} color='red'/>")]
#[case("<Box color='red' {...rest}/>")]
#[serial]
fn explicit_styles_when_spread_order_changes_remain_static(#[case] element: &str) {
    let source = format!("{BOX}export const a=(rest)=>{element};");
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
    assert_eq!(
        whole::evaluate(&source, "a({color:'blue'}).props.color").element,
        "blue"
    );
}

#[rstest]
#[case("{color:'blue'}", "\"blue\"")]
#[case("{color:3}", "3")]
#[case("{color:null}", "null")]
#[case("{}", "undefined")]
#[case("null", "undefined")]
#[case("{color:undefined}", "undefined")]
#[case("Object.create({color:'inherited'})", "undefined")]
#[case("Object.defineProperty({},'color',{value:'hidden'})", "undefined")]
#[serial]
fn raw_props_when_copied_keep_native_values(#[case] rest: &str, #[case] expected: &str) {
    let source = format!("{BOX}export const a=(rest)=><Box color='red' {{...rest}}/>;");
    let actual = whole::evaluate(
        &source,
        &format!(
            "(()=>{{const e=a({rest});return [JSON.stringify(e.props.color)??'undefined',e.props.className.trim()];}})()"
        ),
    );
    assert_eq!(
        actual.element,
        serde_json::json!([expected, "color-0-red--255-a"])
    );
}

#[test]
#[serial]
fn getters_when_spread_props_are_copied_run_once() {
    let source = format!("{BOX}export const a=(rest)=><Box color='red' {{...rest}}/>;");
    let actual = whole::evaluate(
        &source,
        "(()=>{let reads=0;const rest=Object.defineProperty({},'color',{enumerable:true,get(){reads++;return 'x';}});const e=a(rest);return [reads,e.props.color];})()",
    );
    assert_eq!(actual.element, serde_json::json!([1, "x"]));
}

#[test]
#[serial]
fn keys_only_the_spread_gives_stay_untouched() {
    let source = format!(
        "{BOX}export const a=(rest)=><Box color='red' {{...rest}} id='x' title={{rest.t}}/>;"
    );
    let actual = whole::evaluate(&source, "a({color:'blue',t:'title'}).props");
    assert_eq!(actual.element["color"], "blue");
    assert_eq!(actual.element["id"], "x");
    assert_eq!(actual.element["title"], "title");
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
}

#[test]
#[serial]
fn responsive_values_when_spreads_follow_keep_authored_breakpoints() {
    let source = format!(
        "{BOX}export const a=(rest)=><Box bg={{['red','blue',null,'$primary']}} {{...rest}}/>;"
    );
    let mut found: Vec<_> = output(&source)
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(value) => Some((value.level(), value.value().to_string())),
            _ => None,
        })
        .collect();
    found.sort();
    assert_eq!(
        found,
        vec![
            (0, "red".into()),
            (1, "blue".into()),
            (3, "$primary".into())
        ]
    );
    assert_eq!(
        whole::evaluate(&source, "a({bg:'raw'}).props.bg").element,
        "raw"
    );
}

#[test]
#[serial]
fn distinct_authored_values_when_spreads_follow_remain_distinct_classes() {
    let source = format!(
        "{BOX}export const a=(rest)=><><Box color='red' {{...rest}}/><Box color='blue' {{...rest}}/></>;"
    );
    let classes: std::collections::BTreeSet<_> = output(&source)
        .styles
        .iter()
        .filter_map(|style| match style.extract(None) {
            Some(StyleProperty::ClassName(class_name)) => Some(class_name),
            _ => None,
        })
        .collect();
    assert_eq!(classes.len(), 2);
}

#[rstest]
#[case("{}")]
#[case("{ p: 3 }")]
#[case("{ p: undefined }")]
#[case("{ p: null }")]
#[serial]
fn explicit_runtime_values_when_spread_values_differ_remain_authored(#[case] rest: &str) {
    let source = format!("{BOX}export const a=(rest,x)=><Box p={{x}} {{...rest}}/>;");
    assert_eq!(
        whole::evaluate(&source, &format!("Object.values(a({rest},5).props.style)")).element,
        serde_json::json!([5])
    );
}

#[test]
#[serial]
fn conditional_styles_when_spreads_follow_keep_both_branches() {
    let source =
        format!("{BOX}export const a=(rest,on)=><Box color={{on?'red':'blue'}} {{...rest}}/>;");
    assert_eq!(
        static_styles(&output(&source)),
        vec![
            ("color".into(), "blue".into()),
            ("color".into(), "red".into())
        ]
    );
    for (on, color) in [(true, "red"), (false, "blue")] {
        assert!(
            whole::evaluate(
                &source,
                &format!("a({{color:'green'}},{on}).props.className")
            )
            .element
            .as_str()
            .unwrap_or_else(|| panic!("expected an authored class name"))
            .contains(color)
        );
    }
}

#[test]
#[serial]
fn style_order_when_spreads_follow_remains_authored() {
    let source =
        format!("{BOX}export const a=(rest)=><Box color='red' styleOrder={{4}} {{...rest}}/>;");
    assert!(output(&source).styles.iter().all(
        |style| matches!(style,ExtractStyleValue::Static(value) if value.style_order()==Some(4))
    ));
}

#[test]
#[serial]
fn spread_sources_when_selected_or_called_are_read_once() {
    let source = format!("{BOX}export const a=(f)=><Box color='red' {{...f()}}/>;");
    let actual = whole::evaluate(
        &source,
        "a(()=>{trace.push('spread');return {color:'blue'};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread"]));
    assert_eq!(actual.element["props"]["color"], "blue");
    let source = format!("{BOX}export const a=(p)=><Box color='red' {{...(p.on??p.b)}}/>;");
    assert_eq!(
        whole::evaluate(&source, "a({on:null,b:{color:'blue'}}).props.color").element,
        "blue"
    );
    assert_eq!(output(&source).code.matches("p.on ?? p.b").count(), 1);
}

#[test]
#[serial]
fn opaque_literals_when_keys_are_nested_or_computed_forward_them() {
    let source = format!(
        "{BOX}export const a=(rest)=><Box color='red' {{...{{...rest}}}}/>;export const b=(k)=><Box color='red' {{...{{[k]:1}}}}/>;"
    );
    assert_eq!(
        whole::evaluate(
            &source,
            "[a({color:'blue'}).props.color,b('color').props.color]"
        )
        .element,
        serde_json::json!(["blue", 1])
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
}

#[test]
#[serial]
fn non_identifier_keys_when_forwarded_stay_raw() {
    let source = format!("{BOX}export const a=(rest)=><Box background-color='red' {{...rest}}/>;");
    assert_eq!(
        whole::evaluate(
            &source,
            "a({'background-color':'blue'}).props['background-color']"
        )
        .element,
        "blue"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("background-color".into(), "red".into())]
    );
}
