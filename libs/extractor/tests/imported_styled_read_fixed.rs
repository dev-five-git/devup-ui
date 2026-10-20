mod retained;

use retained::{BASE, run, run_with_reading};
use rstest::rstest;
use serial_test::serial;

fn modules(custom: bool, attrs: bool) -> [String; 3] {
    let base = BASE.replace(
        "trace.push('controller')",
        "(trace.push('controller'),order.push('A'))",
    );
    let target = if custom { "Target" } else { "'section'" };
    let middle = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';import {{Base}} from './base';const trace=[];const Target=forwardRef((p,ref)=>jsx('aside',{{...p,ref}}));const Read=styled(Base).withConfig({{shouldForwardProp:p=>p!=='denied'&&p!=='blocked'}})({{color:p=>p.denied,padding:p=>p.$tone,...((trace.push('controller'),order.push('B'),true)?{{color:(trace.push('leaf'),'red')}}:{{}}),color:'green',padding:'2px'}});const Mid=Read.withComponent({target});export {{Mid,trace}};"
    );
    let attrs = if attrs {
        ".attrs(p=>(trace.push([p.denied,p.$tone]),order.push('attrs'),{denied:p.denied+'-outer',$tone:p.$tone+'-outer'}))"
    } else {
        ""
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Mid}} from './mid';const trace=[];const Card=styled(Mid){attrs}({{color:'blue'}});const App=(denied,tone,ref,child)=><Card denied={{denied}} $tone={{tone}} blocked='blocked' ref={{ref}} children={{child}}/>;export {{Card,App,trace}};"
    );
    [base, middle, app]
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn overwritten_prop_reads_keep_fixed_summary_and_original_effect_owners(
    #[case] custom: bool,
    #[values(false, true)] attrs: bool,
) {
    // Given: B's authored prop reads are overwritten, leaving fixed styles and retained effects.
    let [base, middle, app] = modules(custom, attrs);
    // When: the zero-local-style-binding outer hop renders twice, optionally applying local attrs.
    let result = run(
        &[
            ("/src/base.ts", &base),
            ("/src/mid.ts", &middle),
            ("/src/app.tsx", &app),
        ],
        "const app=modules['/src/app.tsx'];const before=[modules['/src/base.ts'].trace.slice(),modules['/src/mid.ts'].trace.slice(),app.trace.slice(),order.slice()];const first=render(app.App('one','tone-one',7,'first'));const second=render(app.App('two','tone-two',8,'second'));JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children,values(e,'color'),values(e,'padding'),slots(e),identities(e).length,new Set(identities(e)).size]),modules['/src/base.ts'].trace,modules['/src/mid.ts'].trace,app.trace,order]);",
    );
    // Then: D1 removes green, both callbacks keep their owners, and outer attrs run only locally.
    let tag = if custom { "aside" } else { "section" };
    let attr_trace = if attrs {
        vec![vec!["one", "tone-one"], vec!["two", "tone-two"]]
    } else {
        vec![]
    };
    let order = if attrs {
        vec!["attrs", "B", "A", "attrs", "B", "A"]
    } else {
        vec!["B", "A", "B", "A"]
    };
    assert_eq!(
        result,
        serde_json::json!([
            [[], [], [], []],
            [
                [tag, 7, "first", ["blue"], ["2px"], [], 3, 3],
                [tag, 8, "second", ["blue"], ["2px"], [], 3, 3]
            ],
            ["controller", "leaf", "controller", "leaf"],
            ["controller", "leaf", "controller", "leaf"],
            attr_trace,
            order
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn denied_and_transient_inputs_reach_read_bearing_callback_before_canonical_filtering(
    #[case] custom: bool,
    #[values(false, true)] attrs: bool,
) {
    // Given: Reads must preserve denied/transient inputs, while Forward still rejects blocked.
    let [base, middle, app] = modules(custom, attrs);
    // When: actual emitted props enter and leave B's original callback across the third module.
    let result = run(
        &[
            ("/src/base.ts", &base),
            ("/src/mid.ts", &middle),
            ("/src/app.tsx", &app),
        ],
        "const raw=modules['/src/app.tsx'].App('caller','transient',7,'child');const final=render(raw);JSON.stringify([[raw.props.denied??null,raw.props.$tone??null,raw.props.blocked??null],frames.slice(0,2).map(e=>[e.props.denied??null,e.props.$tone??null,e.props.blocked??null,slots(e).length]),[final.props.denied??null,final.props.$tone??null,final.props.blocked??null,slots(final),final.props.ref,final.props.children]]);",
    );
    // Then: raw evaluation inputs include local attrs overrides, not the filtered ordinary object.
    let denied = if attrs { "caller-outer" } else { "caller" };
    let tone = if attrs {
        "transient-outer"
    } else {
        "transient"
    };
    assert_eq!(
        result,
        serde_json::json!([
            ["caller", "transient", null],
            [[denied, tone, null, 2], [denied, tone, null, 2]],
            [null, null, null, [], 7, "child"]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn read_bearing_captured_root_does_not_gain_descendant_capability(#[case] readable: bool) {
    // Given: an attrs-free read-bearing ROOT has no emitted pair under the existing root guard.
    let root = "import {styled} from '@devup-ui/react';const trace=[];const Mid=styled.article({color:p=>p.denied,...((trace.push('controller'),true)?{color:(trace.push('leaf'),'red')}:{}),color:'green'});export {Mid,trace};";
    let app = "import {styled} from '@devup-ui/react';import {Mid} from './mid';const Card=styled(Mid)({margin:'3px'});export {Card};";
    // When: a non-overlapping extension renders the original root without acquiring a slot.
    let result = run_with_reading(
        &[("/src/mid.ts", root), ("/src/app.ts", app)],
        "const before=modules['/src/mid.ts'].trace.slice();const final=render(jsx(modules['/src/app.ts'].Card,{ref:7,children:'child'}));JSON.stringify([before,slots(frames[0]),values(final,'color'),values(final,'margin'),final.props.ref,final.props.children,modules['/src/mid.ts'].trace]);",
        readable,
    );
    // Then: proposal A does not widen fixed_native or manufacture a root capability.
    assert_eq!(
        result,
        serde_json::json!([
            [],
            [],
            ["green"],
            ["3px"],
            7,
            "child",
            ["controller", "leaf"]
        ])
    );
}
