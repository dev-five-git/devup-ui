use super::*;

#[rstest]
#[case::renamed("import {css as paint} from '@devup-ui/react';", "paint", false)]
#[case::namespace("import * as ui from '@devup-ui/react';", "ui.styled.div", true)]
#[case::alias("const factory=styled;", "factory.div", true)]
#[serial]
fn semantic_factories_keep_authored_schedule_when_arrays_capture_getters(
    #[case] setup: &str,
    #[case] factory: &str,
    #[case] component: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = format!(
        "{setup}const trace=[];let n=0;const state={{get before(){{trace.push('before');return 'before';}},get on(){{trace.push('on');return ++n===1;}},get middle(){{trace.push('middle');return 'middle';}},get after(){{trace.push('after');return 'after';}}}};const f=input=>{factory}(input.before,input.on?[{{color:'red'}},{{margin:1}}]:[{{color:'blue'}}],input.middle,input.on?[]:[],input.after);const defined=trace.slice();"
    );
    let call = if component {
        "render({target:f(state),props:{}})"
    } else {
        "({props:{className:f(state)}})"
    };
    let observation = format!(
        "const first={call};const firstTrace=trace.slice();const second={call};JSON.stringify([defined,firstTrace,trace,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);"
    );
    // When
    let result = execute_in_mode("integration.ts", &source, &observation, single_css);
    // Then
    assert_eq!(
        result,
        serde_json::json!([
            [],
            ["before", "on", "middle", "on", "after"],
            [
                "before", "on", "middle", "on", "after", "before", "on", "middle", "on", "after"
            ],
            ["red"],
            ["4px"],
            ["blue"],
            []
        ])
    );
}

#[rstest]
#[case::namespace("import * as ui from '@devup-ui/react';", "ui.styled")]
#[case::alias("const factory=styled;", "factory")]
#[serial]
fn callbacks_keep_own_arguments_and_defaults_when_factories_use_semantic_aliases(
    #[case] setup: &str,
    #[case] factory: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = format!(
        "{setup}const trace=[];const Card={factory}('section')(function(p,other=(trace.push('default'),'red')){{return {{color:(trace.push(arguments.length),other),margin:arguments[0].m}};}});const defined=trace.slice();"
    );
    let observation = "const first=render({target:Card,props:{ m: 2 }});const second=render({target:Card,props:{ m: 3 }});JSON.stringify([defined,trace,first.target,values(first,'color'),values(first,'margin'),values(second,'margin')]);";
    // When
    let result = execute_in_mode("integration.ts", &source, observation, single_css);
    // Then
    assert_eq!(
        result,
        serde_json::json!([
            [],
            ["default", 1, "default", 1],
            "section",
            ["red"],
            ["2px"],
            ["3px"]
        ])
    );
}
