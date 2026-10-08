use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    r#"<Box className="direct" style={{opacity:1}} {...{'className':'spread','style':{'opacity':2}}}/>"#,
    "spread",
    2
)]
#[case(
    r#"<Box {...{'className':'spread','style':{'opacity':2}}} className="direct" style={{opacity:1}}/>"#,
    "direct",
    1
)]
#[case(
    r#"<Box className="direct" style={{opacity:1}} {...rest}/>"#,
    "spread",
    2
)]
#[case(
    r#"<Box className="direct" {...{title:'x'}} {...{...rest,className:'last'}}/>"#,
    "last",
    2
)]
#[case(
    r#"<Box className="direct" {...{className:'first',...rest}}/>"#,
    "spread",
    2
)]
#[case(r"<Box {...rest} {...more}/>", "last", 3)]
#[serial]
fn forwarded_props_when_later_values_overwrite_them_match_original_props(
    #[case] element: &str,
    #[case] class: &str,
    #[case] opacity: u8,
) {
    let actual = evaluate(
        &format!("{BOX}export const a=(rest,more)=>{element};"),
        r"a({'className':'spread','style':{'opacity':2}},{'className':'last','style':{'opacity':3}})",
    );
    assert_eq!(actual.element["props"]["className"], class);
    assert_eq!(actual.element["props"]["style"]["opacity"], opacity);
}

#[test]
#[serial]
fn shadowed_object_when_multiple_spreads_select_props_does_not_replace_the_intrinsic() {
    let actual = evaluate(
        &format!(
            "{BOX}export const a=(Object,first,last)=><Box className=\"direct\" style={{{{opacity:1}}}} color=\"red\" {{...first}} {{...last}}/>;"
        ),
        r"a(null,{'className':'first','style':{'opacity':2}},{'className':undefined,'style':undefined})",
    );
    assert!(
        actual.element["props"]["className"]
            .as_str()
            .is_some_and(|class| !class.contains("first") && !class.contains("direct"))
    );
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(|style| !style.contains_key("opacity"))
    );
}

#[test]
#[serial]
fn ordinary_attributes_when_styles_are_lowered_keep_source_order() {
    let source = format!(
        "{BOX}export const a=(take)=><Box id={{take('id')}} color={{take('color')}} className={{take('class')}} style={{take('style')}} title={{take('title')}}>{{take('child')}}</Box>;"
    );
    let actual = evaluate(
        &source,
        r"a(key=>{trace.push(key);return key==='style'?{'opacity':0.5}:key;})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "color", "class", "style", "title", "child"])
    );
}
