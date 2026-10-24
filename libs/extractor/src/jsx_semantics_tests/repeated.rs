use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("{color:'blue'}", "{}", "blue")]
#[case("{color:'blue'}", "{color:'green'}", "green")]
#[case("{}", "{color:'green'}", "green")]
#[case("{}", "{}", "missing")]
#[case("{ p: 2 }", "{}", "missing")]
#[case("{}", "{ p: 2 }", "missing")]
#[serial]
fn repeated_spreads_when_keys_differ_keep_dom_precedence(
    #[case] first: &str,
    #[case] last: &str,
    #[case] color: &str,
) {
    let source =
        format!("{BOX}export const a=(a,b)=><Box color='red' {{...a}} p={{1}} {{...b}}/>;");
    let actual = whole::evaluate(
        &source,
        &format!("a({first},{last}).props.color??'missing'"),
    );
    assert_eq!(actual.element, color);
    assert_eq!(
        static_styles(&output(&source)),
        vec![
            ("color".into(), "red".into()),
            ("padding".into(), "4px".into())
        ]
    );
}

#[rstest]
#[case("{color:undefined}", "undefined")]
#[case("{color:null}", "null")]
#[case("{}", "blue")]
#[serial]
fn own_absence_when_written_later_replaces_only_dom_data(
    #[case] last: &str,
    #[case] expected: &str,
) {
    let source =
        format!("{BOX}export const a=(first,last)=><Box color='red' {{...first}} {{...last}}/>;");
    assert_eq!(
        whole::evaluate(
            &source,
            &format!("String(a({{color:'blue'}},{last}).props.color)")
        )
        .element,
        expected
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
}

#[rstest]
#[case("<Box color='red' {...rest} color='blue'/>", "color", "blue")]
#[case("<Box bg='red' background='blue' {...rest}/>", "background", "blue")]
#[case("<Box {...rest} {...{color:'red'}}/>", "color", "red")]
#[case("<Box {...{color:'red'}} {...rest} color='blue'/>", "color", "blue")]
#[serial]
fn known_writes_when_spreads_interleave_keep_last_authored_value(
    #[case] element: &str,
    #[case] property: &str,
    #[case] value: &str,
) {
    let source = format!("{BOX}export const a=(rest)=>{element};");
    assert_eq!(
        static_styles(&output(&source)),
        vec![(property.into(), value.into())]
    );
    assert_eq!(
        whole::evaluate(&source, "a({color:'raw'}).props.color").element,
        "raw"
    );
}

#[test]
#[serial]
fn known_literal_when_unknown_spread_follows_keeps_its_styles() {
    let source =
        format!("{BOX}export const a=(rest)=><Box {{...{{color:'red',p:1}}}} {{...rest}}/>;");
    assert_eq!(
        static_styles(&output(&source)),
        vec![
            ("color".into(), "red".into()),
            ("padding".into(), "4px".into())
        ]
    );
    let actual = whole::evaluate(&source, "a({color:'blue',margin:1}).props");
    assert_eq!(actual.element["color"], "blue");
    assert_eq!(actual.element["margin"], 1);
}

#[test]
#[serial]
fn nested_opaque_spreads_when_followed_by_another_keep_dom_winners() {
    let source = format!("{BOX}export const a=(a,b)=><Box color='red' {{...{{...a}}}} {{...b}}/>;");
    assert_eq!(
        whole::evaluate(
            &source,
            "[a({color:'x'},{}).props.color,a({color:'x'},{color:'y'}).props.color]"
        )
        .element,
        serde_json::json!(["x", "y"])
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "red".into())]
    );
}

#[test]
#[serial]
fn known_literals_when_styles_are_removed_keep_dom_props() {
    let rendered = code(&format!(
        "{BOX}export const a=(rest)=><Box {{...{{color:'red',id:'x'}}}} {{...rest}}/>;"
    ));
    assert!(rendered.contains("id: \"x\""), "{rendered}");
    assert!(!rendered.contains("color: \"red\""), "{rendered}");
}

#[test]
#[serial]
fn authored_aliases_when_one_target_is_shadowed_keep_the_other() {
    let source = format!("{BOX}export const a=(rest)=><Box mx={{1}} ml={{2}} {{...rest}}/>;");
    assert_eq!(
        static_styles(&output(&source)),
        vec![
            ("margin-left".into(), "8px".into()),
            ("margin-right".into(), "4px".into())
        ]
    );
}
