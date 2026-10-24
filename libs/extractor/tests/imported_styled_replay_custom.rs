mod retained;

use retained::{BASE as CAPTURED_BASE, run, run_with_reading};
use rstest::rstest;
use serial_test::serial;

const BASE: &str = "import {styled} from '@devup-ui/react';const Base=styled.article({color:'green',padding:'2px'});export {Base};";
const TARGET: &str = "const trace=[];const Target=forwardRef((p,ref)=>{const {theme,customProp,...ordinary}=p;return jsx('aside',{...ordinary,ref,title:theme?.tone,'data-custom':customProp});});const targets={get Target(){trace.push('target');return Target;}};";

#[rstest]
#[case("const Mid=Base.withComponent(targets.Target);")]
#[case(
    "const Retarget=Base.withComponent(targets.Target);const Mid=styled(Retarget)({margin:'1px'});"
)]
#[case(
    "const Retarget=Base.withComponent(targets.Target);const Mid=styled(Retarget)`margin:1px;`;"
)]
#[serial]
fn fixed_custom_replay_descendant_retains_renderer_when_imported_styles_replace_green(
    #[case] definition: &str,
    #[values(false, true)] captured: bool,
    #[values(false, true)] collide: bool,
) {
    // Given: binding-free native A flattens into custom B, whose target belongs only to B.
    let authored = if collide {
        "const authored={['__devupCss0' as const]:'class',[('__devupForwarded0' as string)]:'forwarded'};"
    } else {
        ""
    };
    let middle = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';import {{Base}} from './base';{authored}{TARGET}{definition}export {{Mid,trace}};"
    );
    let effects = if captured {
        "...((trace.push('controller'),order.push('C'),true)?{margin:(trace.push('leaf'),'1px')}:{}),"
    } else {
        ""
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{Mid}} from './mid';const trace=[];const Card=styled(Mid)({{{effects}margin:'2px',color:'blue'}});const App=(ref,children)=> <Card css={{{{padding:5}}}} ref={{ref}} theme={{{{tone:'caller'}}}} customProp='custom' style={{{{borderColor:'orange'}}}}>{{children}}</Card>;export {{App,Card,trace}};"
    );
    // When: the exact public source renders twice, then an explicit target bypasses B's getter.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", &middle),
            ("/src/app.tsx", &app),
        ],
        "const mid=modules['/src/mid.ts'];const app=modules['/src/app.tsx'];const before=[mid.trace.slice(),app.trace.slice()];const first=render(app.App(7,'first'));const incoming=frames[0];const original=incoming.target===mid.Mid;const flattened=frames.every(e=>e.target!==modules['/src/base.ts'].Base);const controls=slots(incoming);const second=render(app.App(8,'second'));const third=render(jsx(app.Card,{as:'nav',forwardedAs:'em',ref:9,children:'third',style:{borderColor:'orange'}}));JSON.stringify([before,original,flattened,controls.length,controls.includes('__devupCss0'),controls.includes('__devupForwarded0'),[first,second,third].map(e=>[e.target,e.props.ref,e.props.children,e.props.as,values(e,'color'),values(e,'padding'),values(e,'margin'),slots(e),e.props.style.borderColor,identities(e).length,new Set(identities(e)).size]),first.props.title,first.props['data-custom'],mid.trace,app.trace,order]);",
    );
    // Then: real B consumes the pair; only blue/5px remain, without replaying A's callback.
    let effects = if captured {
        vec![
            "controller",
            "leaf",
            "controller",
            "leaf",
            "controller",
            "leaf",
        ]
    } else {
        vec![]
    };
    assert_eq!(
        result,
        serde_json::json!([
            [[], []],
            true,
            true,
            2,
            !collide,
            !collide,
            [
                [
                    "aside",
                    7,
                    "first",
                    null,
                    ["blue"],
                    ["5px"],
                    ["2px"],
                    [],
                    "orange",
                    3,
                    3
                ],
                [
                    "aside",
                    8,
                    "second",
                    null,
                    ["blue"],
                    ["5px"],
                    ["2px"],
                    [],
                    "orange",
                    3,
                    3
                ],
                [
                    "nav",
                    9,
                    "third",
                    "em",
                    ["blue"],
                    ["2px"],
                    ["2px"],
                    [],
                    "orange",
                    3,
                    3
                ]
            ],
            "caller",
            "custom",
            ["target", "target"],
            effects,
            if captured {
                vec!["C", "C", "C"]
            } else {
                vec![]
            }
        ])
    );
}

#[rstest]
#[case(
    "/src/app.ts",
    "jsx(Mid,{css:{color:'blue',padding:5},theme:{tone:'caller'},ref:7,children:'child'})"
)]
#[case(
    "/src/app.tsx",
    "<Mid css={{color:'blue',padding:5}} theme={{tone:'caller'}} ref={7}>child</Mid>"
)]
#[serial]
fn direct_css_uses_real_custom_descendant_pair_when_base_is_binding_free(
    #[case] filename: &str,
    #[case] element: &str,
) {
    // Given: direct imported JSX and JSX-call consumers share the same fixed custom descendant.
    let middle = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';import {{Base}} from './base';{TARGET}const Mid=Base.withComponent(targets.Target);export {{Mid,trace}};"
    );
    let app = format!(
        "import {{jsx}} from '@emotion/react';import {{Mid}} from './mid';const App=()=>{element};export {{App}};"
    );
    // When: a caller override replaces the imported component's fixed defaults.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", &middle),
            (filename, &app),
        ],
        &format!(
            "const mid=modules['/src/mid.ts'];const raw=modules[{filename:?}].App();const final=render(raw);JSON.stringify([raw.target===mid.Mid,slots(raw).length,final.target,final.props.title,final.props.ref,final.props.children,values(final,'color'),values(final,'padding'),slots(final),mid.trace]);"
        ),
    );
    // Then: both syntax paths replace green through the original custom renderer.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            1,
            "aside",
            "caller",
            7,
            "child",
            ["blue"],
            ["5px"],
            [],
            ["target"]
        ])
    );
}

#[rstest]
#[case(("const Mid=styled(Target)({color:'green',padding:'2px'});", "2px", None, false), false)]
#[case(("const Retarget=Base.withComponent(Target);const Mid=styled(Retarget).attrs(p=>({id:p.source}))({});", "2px", Some("source"), false), true)]
#[case(("const Retarget=Base.withComponent(Target);const Mid=styled(Retarget)`padding:${p=>p.space};`;", "4px", None, false), true)]
#[case(("const Retarget=Base.withComponent(Target);const Mid=styled(Retarget)('opaque',{});", "2px", None, true), true)]
#[serial]
fn excluded_custom_definitions_keep_existing_behavior_without_descendant_capability(
    #[case] fixture: (&str, &str, Option<&str>, bool),
    #[case] staged: bool,
    #[values(false, true)] readable: bool,
) {
    let (definition, padding, id, opaque) = fixture;
    // Given: these definitions stay excluded from fixed complete-replacement pairs.
    let base = if readable {
        "import {Base} from './base';"
    } else {
        "const Base=styled.article({color:'green',padding:'2px'});"
    };
    let middle = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';{base}{TARGET}{definition}export {{Mid}};"
    );
    let app = "import {styled} from '@devup-ui/react';import {Mid} from './mid';const Card=styled(Mid)({margin:'3px'});export {Card};";
    // When: a non-overlapping extension renders through the unchanged readable/unread boundary.
    let result = run_with_reading(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", &middle),
            ("/src/app.ts", app),
        ],
        "const final=render(jsx(modules['/src/app.ts'].Card,{source:'source',space:'4px',ref:7,children:'child'}));const incoming=frames[0];const controls=slots(incoming);const stages=controls.filter(k=>Array.isArray(incoming.props[k]));JSON.stringify([controls.filter(k=>!Array.isArray(incoming.props[k])).length,final.target,values(final,'color'),values(final,'padding'),values(final,'margin'),final.props.id??null,final.props.ref,final.props.children,slots(final),final.props.className.split(' ').includes('opaque'),stages.map(k=>[incoming.props[k].length,...incoming.props[k].map(value=>typeof value)])]);",
        readable,
    );
    // Then: only approved residual stages cross the boundary, never a fixed pair or leaked control.
    assert_eq!(
        result,
        serde_json::json!([
            0,
            "aside",
            ["green"],
            [padding],
            ["3px"],
            id,
            7,
            "child",
            [],
            opaque,
            if readable && staged {
                serde_json::json!([[4, "function", "function", "boolean", "undefined"]])
            } else {
                serde_json::json!([])
            }
        ])
    );
}

#[rstest]
#[case(BASE, false)]
#[case(CAPTURED_BASE, true)]
#[serial]
fn empty_pair_is_complete_when_fixed_custom_descendant_has_replay_or_retained_base(
    #[case] base: &str,
    #[case] captured: bool,
) {
    // Given: custom B is derived from either the approved Replay or existing captured root.
    let middle = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';import {{Base}} from './base';{TARGET}const Mid=Base.withComponent(targets.Target);export {{Mid,trace}};"
    );
    let app = "import {styled} from '@devup-ui/react';import {Mid} from './mid';const Card=styled(Mid)({color:'blue'});export {Card};";
    // When: the emitted exact pair is used to send empty class and forwarding replacements.
    let result = run(
        &[
            ("/src/base.ts", base),
            ("/src/mid.ts", &middle),
            ("/src/app.ts", app),
        ],
        "render(jsx(modules['/src/app.ts'].Card,{}));const controls=slots(frames[0]);const classKey=controls.find(k=>k.startsWith('__devupCss'));const forwardedKey=controls.find(k=>k.startsWith('__devupForwarded'));const mid=modules['/src/mid.ts'];const final=render(jsx(mid.Mid,{[classKey]:'',[forwardedKey]:{},ref:7,children:'removed',theme:{tone:'removed'}}));JSON.stringify([controls.length,final.target,values(final,'color'),values(final,'padding'),final.props.ref??null,final.props.children??null,final.props.title??null,slots(final),mid.trace,modules['/src/base.ts'].trace??[]]);",
    );
    // Then: no default is restored, and retained captures still execute only in their owner.
    assert_eq!(
        result,
        serde_json::json!([
            2,
            "aside",
            [],
            [],
            null,
            null,
            null,
            [],
            ["target", "target"],
            if captured {
                vec!["controller", "leaf", "controller", "leaf"]
            } else {
                vec![]
            }
        ])
    );
}
