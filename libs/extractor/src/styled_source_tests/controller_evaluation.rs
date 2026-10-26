use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::emotion("emotionCss", false, "1px")]
#[case::native("css", false, "4px")]
#[case::styled("styled.div", true, "4px")]
#[serial]
fn authored_controllers_keep_distinct_reads_between_opaque_classes(
    #[case] factory: &str,
    #[case] styled: bool,
    #[case] margin: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = format!(
        "import {{css as emotionCss}} from '@emotion/react';const trace=[];let n=0;const state={{get before(){{trace.push('before');return 'before';}},get on(){{trace.push('read');return ++n===1;}},get middle(){{trace.push('middle');return 'middle';}},get after(){{trace.push('after');return 'after';}}}};const f=input=>{factory}(input.before,input.on?[{{color:'red'}},{{margin:1}}]:[{{color:'blue'}}],input.middle,input.on?[{{background:'green'}},{{padding:1}}]:[{{background:'black'}}],input.after);"
    );
    let call = if styled {
        "render({target:f(state),props:{}})"
    } else {
        "({target:'div',props:{className:f(state)}})"
    };
    let observation = format!(
        "const first={call};JSON.stringify([trace,values(first,'color'),values(first,'margin'),values(first,'background'),values(first,'padding'),['before','middle','after'].filter(name=>first.props.className.split(/\\s+/).includes(name))]);"
    );
    // When
    let observed = execute_in_mode("test.ts", &source, &observation, single_css);
    // Then
    assert_eq!(
        observed,
        serde_json::json!([
            ["before", "read", "middle", "read", "after"],
            ["red"],
            [margin],
            ["black"],
            [],
            ["before", "middle", "after"]
        ])
    );
}

#[rstest]
#[serial]
fn styled_controllers_stay_per_render_with_short_circuit_guards(
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const trace=[];let selected=true;const state={get outer(){trace.push('outer');return selected;},get inner(){trace.push('inner');return true;}};const Card=styled.div(state.outer&&state.inner?[{color:'red'},{margin:1}]:[{color:'blue'}]);const defined=trace.slice();";
    let observation = "const first=render({target:Card,props:{}});const firstTrace=trace.slice();selected=false;const second=render({target:Card,props:{}});JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);";
    // When
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["outer", "inner"],
            ["outer", "inner", "outer"],
            ["red"],
            ["4px"],
            ["blue"],
            []
        ])
    );
}

#[rstest]
#[serial]
fn styled_leaf_reads_keep_authored_order_even_when_later_rules_cover_them(
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const trace=[];const state={get before(){trace.push('before');return 'green';},get on(){trace.push('on');return true;},get margin(){trace.push('margin');return '4px';},get after(){trace.push('after');return 0.5;}};const Card=styled.div([{color:state.before},state.on?[{margin:state.margin},{color:'red'}]:[{color:'blue'}],{opacity:state.after}]);";
    let observation = "const first=render({target:Card,props:{}});JSON.stringify([trace,values(first,'color'),values(first,'margin'),values(first,'opacity')]);";
    // When
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then
    assert_eq!(
        observed,
        serde_json::json!([["before", "on", "margin", "after"], ["red"], ["4px"], [0.5]])
    );
}

#[rstest]
#[case::emotion("emotionCss", "1px")]
#[case::native("css", "4px")]
#[serial]
fn conditional_lookup_atoms_keep_guarded_selector_reads(
    #[case] factory: &str,
    #[case] margin: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = format!(
        "import {{css as emotionCss}} from '@emotion/react';const trace=[];let selected=true;const state={{get on(){{trace.push('on');return selected;}},get key(){{trace.push('key');return 'red';}}}};const f=()=>{factory}(state.on?[{{color:{{red:'red',blue:'blue'}}[state.key]}},{{margin:1}}]:[{{color:'black'}}]);"
    );
    let observation = "const first={props:{className:f()}};const firstTrace=trace.slice();selected=false;const second={props:{className:f()}};JSON.stringify([firstTrace,trace,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);";
    // When
    let observed = execute_in_mode("test.ts", &source, observation, single_css);
    // Then
    assert_eq!(
        observed,
        serde_json::json!([
            ["on", "key"],
            ["on", "key", "on"],
            ["red"],
            [margin],
            ["black"],
            []
        ])
    );
}

#[rstest]
#[case::emotion("emotionCss", false)]
#[case::native("css", false)]
#[case::styled("styled.div", true)]
#[serial]
fn empty_rule_arrays_still_evaluate_the_authored_controller(
    #[case] factory: &str,
    #[case] styled: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = format!(
        "import {{css as emotionCss}} from '@emotion/react';const trace=[];const state={{get on(){{trace.push('read');return true;}}}};const f=()=>{factory}(state.on?[]:[]);"
    );
    let observation = if styled {
        "render({target:f(),props:{}});JSON.stringify(trace);"
    } else {
        "f();JSON.stringify(trace);"
    };
    // When
    let observed = execute_in_mode("test.ts", &source, observation, single_css);
    // Then
    assert_eq!(observed, serde_json::json!(["read"]));
}

#[rstest]
#[serial]
fn styled_extensions_rebind_captured_classes_with_their_controllers(
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const trace=[];let n=0;const state={get opaque(){trace.push('class');return 'opaque';},get on(){trace.push('on');return ++n===1;}};const f=input=>{const Base=styled.div(input.opaque,input.on?[{color:'red'},{margin:1}]:[{color:'blue'}]);const With=Base.withComponent('section');const Child=styled(Base)({padding:1});return {With,Child};};const {With,Child}=f(state);const defined=trace.slice();";
    let observation = "const first=render({target:With,props:{}});const second=render({target:Child,props:{}});JSON.stringify([defined,trace,first.target,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin'),values(second,'padding'),[first,second].map(element=>element.props.className.split(/\\s+/).includes('opaque'))]);";
    // When
    let observed = execute_in_mode("test.ts", source, observation, single_css);
    // Then
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["class", "on", "class", "on"],
            "section",
            ["red"],
            ["4px"],
            ["blue"],
            [],
            ["4px"],
            [true, true]
        ])
    );
}

#[path = "controller_evaluation_refusals.rs"]
mod refusals;
