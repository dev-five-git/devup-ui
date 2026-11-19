use super::*;

#[rstest]
#[case("<Box px={[null,1,2,3]} {...rest}/>")]
#[case("jsx(Box,{px:[null,1,2,3],...rest})")]
#[case("jsx(Box,{as:tag,px:[null,1,2,3],...rest})")]
#[serial]
fn grouped_spread_when_frontdoors_expand_targets_owns_one_authored_boundary(#[case] element: &str) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest,tag)=>{element};");
    let compiled = output(&source);
    assert_eq!(
        compiled.code.matches("\"px\" in ").count(),
        0,
        "{}",
        compiled.code
    );
    assert_eq!(
        compiled.code.matches(".test(").count(),
        0,
        "{}",
        compiled.code
    );
    let actual = whole::evaluate(
        &source,
        "a({get px(){trace.push('get');return 2;}},'section')",
    );
    assert_eq!(actual.trace, serde_json::json!(["get"]));
    assert_eq!(
        selected(&source, "a({ px:2 },'section')"),
        serde_json::json!([null, "4px", "8px", "12px"])
    );
}

#[rstest]
#[case("<Box px={[null,1,2]} {...rest} pl={7}/>")]
#[case("<Box px={[null,1,2]} pl={7} {...rest}/>")]
#[case("jsx(Box,{px:[null,1,2],...rest,pl:7})")]
#[case("jsx(Box,{px:[null,1,2],pl:7,...rest})")]
#[serial]
fn grouped_spread_when_a_later_alias_shadows_half_keeps_only_surviving_targets(
    #[case] element: &str,
) {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest)=>{element};");
    let compiled = output(&source);
    let winners: Vec<_> = compiled
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(value) if value.property() == "padding-right" => {
                Some((value.property(), value.level(), value.value()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        winners,
        vec![("padding-right", 1, "4px"), ("padding-right", 2, "8px")],
        "{}",
        compiled.code
    );
    assert!(static_styles(&compiled).contains(&("padding-left".into(), "28px".into())));
    assert_eq!(compiled.code.matches("\"px\" in ").count(), 0);
    assert_eq!(compiled.code.matches(".test(").count(), 0);
}

#[test]
#[serial]
fn grouped_spread_when_the_key_is_written_again_preserves_the_later_boundary_and_effects() {
    let source = format!(
        "{BOX}export const a=(first,last)=><Box px={{(trace.push('old'),[1,2])}} {{...first}} px={{(trace.push('new'),[null,3])}} {{...last}} pr={{7}}/>;"
    );
    let compiled = output(&source);
    let actual = whole::evaluate(
        &source,
        "a({get px(){trace.push('first');return 9;}},{get px(){trace.push('last');return 2;}})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["old", "first", "new", "last"])
    );
    assert_eq!(compiled.code.matches("\"px\" in ").count(), 0);
    assert_eq!(compiled.code.matches(".test(").count(), 0);
    assert_eq!(
        static_styles(&compiled),
        vec![("padding-right".into(), "28px".into())]
    );
    let control = whole::evaluate(
        &source.replace(" {...first}", "").replace(" {...last}", ""),
        "a({}, {})",
    );
    assert_eq!(
        actual.element["props"]["style"],
        control.element["props"]["style"]
    );
    assert_eq!(actual.element["props"]["px"], 2);
}
