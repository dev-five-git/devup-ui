use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::standalone("{color:'red',[props.key]:'blue'}", false)]
#[case::multipart("[{color:'red',[props.key]:'blue'},{color:'green'}]", false)]
#[case::conditional("props.on?{color:'red',[props.key]:'blue'}:{color:'green'}", false)]
#[case::covering("[{color:'red',[props.key]:'blue'},{color:'green'}]", true)]
#[serial]
fn d1_boundary_reports_computed_key_when_valid_or_covering_children_exist(
    #[case] rules: &str,
    #[case] covering: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: unreadable syntax occurs beside valid declarations, possibly overwritten later.
    let class_name = if covering { " className={cover}" } else { "" };
    let source = format!(
        "import {{css}} from '@emotion/react';const cover=css({{color:'black'}});export function App(props){{return <div css={{{rules}}}{class_name}/>;}}"
    );
    let option = crate::ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..crate::ExtractOption::default()
    };
    // When: the real extractor admits and diagnoses the complete CSS prop.
    let result = crate::extract("boundary-diagnostic.tsx", &source, option);
    // Then: the raw source's computed key is refused even if composition could discard it.
    let error = result
        .err()
        .unwrap_or_else(|| panic!("computed key unexpectedly compiled"));
    let error = error.to_string();
    assert!(error.starts_with("boundary-diagnostic.tsx:1:"), "{error}");
    assert!(error.contains("[props.key]"), "{error}");
}

#[rstest]
#[case::class_cover(
    "const cover=css({p:'4px'});const App=()=>jsx('div',{css:{padding:'8px'},className:cover});"
)]
#[case::inherited(
    "const Base=styled.div({color:'black'});const App=()=>jsx(Base,{css:{padding:'8px',p:'4px'}});"
)]
#[case::known_composition(
    "const mixin=css({color:'black'});const selected=css(mixin,{padding:'8px',p:'4px'});const App=()=>jsx('div',{className:selected});"
)]
#[serial]
fn d1_boundary_emits_only_composed_winner_when_other_declarations_participate(
    #[case] source: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: known class coverage, inherited declarations or known-class composition forbids raw publication.
    // When: the complete generated renderer selects classes from the emitted atom map.
    let observed = execute_in_mode(
        "boundary-emission.ts",
        source,
        "const element=render(App());JSON.stringify([values(element,'padding'),Object.values(rules).filter(r=>r[0]==='padding').map(r=>r[1]).sort()]);",
        single_css,
    );
    // Then: neither selection nor global emitted padding contains the losing raw8px.
    assert_eq!(observed, serde_json::json!([["4px"], ["4px"]]));
}

#[rstest]
#[case::arguments("{padding:'8px'},{p:'4px'}", false)]
#[case::array("[{padding:'8px'},{p:'4px'}]", false)]
#[case::spread("...[{padding:'8px'},{p:'4px'}]", false)]
#[case::nested("[[{padding:'8px'}],{p:'4px'}]", false)]
#[case::conditional("true?[{padding:'8px'},{p:'4px'}]:[]", false)]
#[case::opaque("{padding:'8px'},'external',{p:'4px'}", true)]
#[serial]
fn d1_boundary_selects_later_alias_when_parts_arrive_through_rule_containers(
    #[case] arguments: &str,
    #[case] opaque: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: genuine authored parts have not yet been merged by literal property spelling.
    let source = format!("const Card=styled.div({arguments});");
    // When: the real renderer uses the shared authored projection for composition.
    let observed = execute_in_mode(
        "boundary-containers.ts",
        &source,
        "const element=render({target:Card,props:{}});JSON.stringify([values(element,'padding'),element.props.className.split(/\\s+/).includes('external')]);",
        single_css,
    );
    // Then: the later4px is the only selected declaration and opaque classes retain their identity.
    assert_eq!(observed, serde_json::json!([["4px"], opaque]));
}

#[rstest]
#[serial]
fn d1_boundary_keeps_one_callback_schedule_when_all_earlier_padding_is_covered(
    #[values(false, true)] single_css: bool,
) {
    // Given: a covered callback and its argument siblings expose their authored evaluation order.
    let source = "const trace=[];const state={get before(){trace.push('before');return '8px';},get after(){trace.push('after');return '4px';}};const Card=styled.div({padding:state.before},(p,unused=(trace.push('default'),0))=>({p:(trace.push('leaf'),p.padding)}),{p:state.after});const defined=trace.slice();";
    // When: the generated component renders once through the original callback pipeline.
    let observed = execute_in_mode(
        "boundary-schedule.ts",
        source,
        "const element=render({target:Card,props:{padding:'16px'}});JSON.stringify([defined,trace,values(element,'padding')]);",
        single_css,
    );
    // Then: declaration removal never removes or repeats the authored argument evaluations.
    assert_eq!(
        observed,
        serde_json::json!([[], ["before", "default", "leaf", "after"], ["4px"]])
    );
}
