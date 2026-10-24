use super::*;

#[rstest]
#[case("[null,1]", [None, Some("4px"), Some("4px"), Some("4px")], [None, Some("4px"), Some("4px"), Some("4px")])]
#[case("on ? [null,1] : [2,null,3]", [None, Some("4px"), Some("4px"), Some("4px")], [Some("8px"), Some("8px"), Some("12px"), Some("12px")])]
#[case("on ? (first.enabled ? null : null) : '4px'", [None; 4], [Some("4px"); 4])]
#[case("on ? (first.enabled ? null : null) : [null,value()]", [None; 4], [None, Some("5px"), Some("5px"), Some("5px")])]
#[case("on ? [value(),null,2] : [null,'12px']", [Some("5px"), Some("5px"), Some("8px"), Some("8px")], [None, Some("12px"), Some("12px"), Some("12px")])]
#[serial]
fn authored_coverage_when_originals_have_holes_keeps_them(
    #[case] original: &str,
    #[case] yes: [Option<&str>; 4],
    #[case] no: [Option<&str>; 4],
) {
    let source = format!(
        "{BOX}export const a=(rest,on)=>{{const first={{enabled:true}};const value=()=> '5px';return <Box p={{{original}}} {{...rest}}/>;}};"
    );
    for (on, missing) in [(true, yes), (false, no)] {
        for input in [
            "{}",
            "{ p: 2 }",
            "{ p: 0 }",
            "{p:'12px'}",
            "{ p: undefined }",
            "{ p: null }",
        ] {
            let actual = selected(&source, &format!("a({input},{on})"));
            assert_eq!(
                actual,
                serde_json::json!(missing),
                "{original}/{on}/{input}"
            );
        }
    }
}

#[test]
#[serial]
fn raw_variable_when_unresolved_cannot_replace_authored_css() {
    let source = format!("{BOX}export const a=(rest)=><Box p={{1}} {{...rest}}/>;");
    let compiled = output(&source);
    let element = whole::evaluate(&source, "a({p:'var(--unresolved-proof)'})").element;
    assert_eq!(element["props"]["p"], "var(--unresolved-proof)");
    assert_eq!(
        static_styles(&compiled),
        vec![("padding".into(), "4px".into())]
    );
    assert_eq!(
        super::selected(&source, "a({p:'var(--unresolved-proof)'})"),
        serde_json::json!(vec!["4px"; 4])
    );
    assert_eq!(
        super::selected(&source, "a({})"),
        serde_json::json!(vec!["4px"; 4])
    );
}

#[rstest]
#[case("<Box p={value()} {...first} {...last} id={tick('id')}/>")]
#[case("jsx(Box,{p:value(),...first,...last,id:tick('id')})")]
#[case("jsx(Box,{as:tag,p:value(),...first,...last,id:tick('id')})")]
#[serial]
fn spread_boundary_when_frontdoors_capture_effects_both_arms_keep_the_authored_order(
    #[case] element: &str,
) {
    let source = format!(
        "{BOX}import {{jsx}} from 'react/jsx-runtime';export const a=(own)=>{{const tick=x=>(trace.push(x),x);const tag='section';const value=()=> (tick('value'),'5px');const first={{get p(){{tick('first');return 2;}}}};const last=own?{{get p(){{tick('last');return undefined;}}}}:{{get title(){{tick('last');return 'x';}}}};return {element};}};"
    );
    for own in [true, false] {
        let actual = whole::evaluate(&source, &format!("a({own})"));
        assert_eq!(
            actual.trace,
            serde_json::json!(["value", "first", "last", "id"])
        );
        let expected = [Some("5px"); 4];
        assert_eq!(
            selected(&source, &format!("a({own})")),
            serde_json::json!(expected)
        );
    }
}

#[test]
#[serial]
fn spread_boundary_when_nested_elements_reuse_user_names_keeps_frames_isolated() {
    let source = [BOX, "export const a=(rest)=>{const __devupOwn0=17,__devupRaw0=19,__devupPresent0=23,__devupConverted0=29;const h0=31,r0=37,p0=41,v0=43;return <Box p={[null,1]} {...rest}><Box px={[null,2]} {...{get px(){trace.push('inner');return 3;}}}/>{[__devupOwn0,__devupRaw0,__devupPresent0,__devupConverted0]}{[h0,r0,p0,v0]}</Box>;};"].concat();
    let actual = whole::evaluate(&source, "a({})");
    assert_eq!(actual.trace, serde_json::json!(["inner"]));
    assert_eq!(actual.element["children"][0]["props"]["px"], 3);
    assert_eq!(
        static_styles(&output(&source))
            .iter()
            .filter(|(property, value)| property.starts_with("padding-") && value == "8px")
            .count(),
        2
    );
    assert_eq!(
        actual.element["children"][1],
        serde_json::json!([17, 19, 23, 29])
    );
    assert_eq!(
        actual.element["children"][2],
        serde_json::json!([31, 37, 41, 43])
    );
}

#[rstest]
#[case(
    "<Box p={(trace.push('test'),on)?[null,value('left')]:[value('right'),null,3]} {...rest} id={(trace.push('id'),'x')}/>"
)]
#[case(
    "jsx(Box,{p:(trace.push('test'),on)?[null,value('left')]:[value('right'),null,3],...rest,id:(trace.push('id'),'x')})"
)]
#[serial]
fn spread_boundary_when_missing_or_supplied_keeps_original_conditional_effects(
    #[case] element: &str,
) {
    let source = format!(
        "{BOX}import {{jsx}} from 'react/jsx-runtime';export const a=(own,on)=>{{const value=x=>(trace.push(x),x==='left'?'5px':'6px');const rest=own?{{get p(){{trace.push('spread');return 2;}}}}:{{get title(){{trace.push('spread');return 'title';}}}};return {element};}};"
    );
    for own in [true, false] {
        for on in [true, false] {
            let actual = whole::evaluate(&source, &format!("a({own},{on})"));
            assert_eq!(
                actual.trace,
                serde_json::json!(["test", if on { "left" } else { "right" }, "spread", "id"])
            );
            let expected = if on {
                [None, Some("5px"), Some("5px"), Some("5px")]
            } else {
                [Some("6px"), Some("6px"), Some("12px"), Some("12px")]
            };
            assert_eq!(
                selected(&source, &format!("a({own},{on})")),
                serde_json::json!(expected)
            );
        }
    }
}

#[test]
#[serial]
fn authored_targets_when_responsive_keep_every_level_without_conversion() {
    let source = format!("{BOX}export const a=(rest)=><Box px={{[null,1,2,3]}} {{...rest}}/>;");
    let compiled = output(&source);
    let targets: std::collections::BTreeSet<_> = compiled
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(value) => {
                Some((value.property(), value.level(), value.value()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        targets,
        std::collections::BTreeSet::from([
            ("padding-left", 1, "4px"),
            ("padding-left", 2, "8px"),
            ("padding-left", 3, "12px"),
            ("padding-right", 1, "4px"),
            ("padding-right", 2, "8px"),
            ("padding-right", 3, "12px")
        ])
    );
    assert_eq!(compiled.code.matches(".test(").count(), 0);
    assert_eq!(
        compiled.code.matches("\"px\" in ").count(),
        0,
        "{}",
        compiled.code
    );
}
