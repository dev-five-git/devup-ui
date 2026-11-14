mod retained;

use retained::{BASE, RunOptions, run, run_with_options};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn retained_effect_order_matches_winning_styles_when_both_modules_capture(#[case] overlap: bool) {
    // Given: overwritten effects in each module have independent trace owners.
    let base = BASE.replace(
        "trace.push('controller')",
        "(trace.push('controller'),order.push('A'))",
    );
    let color = if overlap { ",color:'blue'" } else { "" };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Base}} from './base';const trace=[];const Card=styled(Base)({{...((trace.push('controller'),order.push('B'),true)?{{margin:(trace.push('leaf'),'1px')}}:{{}}),margin:'2px',padding:'2px'{color}}});export {{Card,trace}};"
    );
    // When: two renders execute the generated callbacks and expose their actual atoms.
    let result = run(
        &[("/src/base.ts", &base), ("/src/app.ts", &app)],
        "const app=modules['/src/app.ts'];const before=[modules['/src/base.ts'].trace.slice(),app.trace.slice()];const first=render(jsx(app.Card,{ref:7,children:'first'}));const second=render(jsx(app.Card,{ref:8,children:'second'}));JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children,values(e,'color'),values(e,'margin'),values(e,'padding'),slots(e)]),modules['/src/base.ts'].trace,app.trace,order]);",
    );
    // Then: B precedes A per render while D1 removes only the losing style, not its effects.
    assert_eq!(
        result,
        serde_json::json!([
            [[], []],
            [
                [
                    "article",
                    7,
                    "first",
                    [if overlap { "blue" } else { "green" }],
                    ["2px"],
                    ["2px"],
                    []
                ],
                [
                    "article",
                    8,
                    "second",
                    [if overlap { "blue" } else { "green" }],
                    ["2px"],
                    ["2px"],
                    []
                ]
            ],
            ["controller", "leaf", "controller", "leaf"],
            ["controller", "leaf", "controller", "leaf"],
            ["B", "A", "B", "A"]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn default_target_reads_stay_in_definer_when_retained_child_renders(#[case] retarget: bool) {
    // Given: the middle module alone owns an observable default-target getter.
    let middle = "import {styled} from '@devup-ui/react';import {forwardRef} from 'react';import {jsx} from 'react/jsx-runtime';import {Base} from './base';const trace=[];const Target=forwardRef((p,ref)=>jsx('aside',{...p,ref}));const targets={get Target(){trace.push('target');return Target;}};const Mid=Base.withComponent(targets.Target);export {Mid,trace};";
    let definition = if retarget {
        "Mid.withComponent('main')"
    } else {
        "styled(Mid)({})"
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Mid}} from './mid';const Card={definition};export {{Card}};"
    );
    // When: two default renders and one runtime-as override traverse the original middle callback.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", middle),
            ("/src/app.ts", &app),
        ],
        "const mid=modules['/src/mid.ts'];const before=mid.trace.slice();const Card=modules['/src/app.ts'].Card;const first=render(jsx(Card,{ref:7,children:'first'}));const second=render(jsx(Card,{ref:8,children:'second'}));const third=render(jsx(Card,{as:'nav',forwardedAs:'em',ref:9,children:'third'}));JSON.stringify([before,[first,second,third].map(e=>[e.target,e.props.ref,e.props.children,e.props.as??null,slots(e)]),mid.trace,modules['/src/base.ts'].trace]);",
    );
    // Then: no eager or importer getter read is added; explicit targets skip obsolete defaults.
    let tag = if retarget { "main" } else { "aside" };
    let reads = if retarget {
        vec![]
    } else {
        vec!["target", "target"]
    };
    assert_eq!(
        result,
        serde_json::json!([
            [],
            [
                [tag, 7, "first", null, []],
                [tag, 8, "second", null, []],
                ["nav", 9, "third", "em", []]
            ],
            reads,
            [
                "controller",
                "leaf",
                "controller",
                "leaf",
                "controller",
                "leaf"
            ]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn custom_export_advertises_its_own_slot_when_imported_css_replaces_styles(#[case] jsx: bool) {
    // Given: a custom fixed-summary descendant has no local captures or portable Target AST.
    let middle = "import {styled} from '@devup-ui/react';import {forwardRef} from 'react';import { jsx } from 'react/jsx-runtime';import {Base} from './base';const Target=forwardRef((p,ref)=>jsx('aside',{...p,'data-tone':p.theme?.tone,ref}));const Mid=Base.withComponent(Target);export {Mid};";
    let filename = if jsx { "/src/app.tsx" } else { "/src/app.ts" };
    let element = if jsx {
        "<Mid css={{color:'blue',padding:'4px'}} theme={{tone:'caller'}} ref={7} children='child'/>"
    } else {
        "jsx(Mid,{css:{color:'blue',padding:'4px'},theme:{tone:'caller'},ref:7,children:'child'})"
    };
    let app = format!(
        "import {{jsx}} from '@emotion/react';import {{Mid}} from './mid';const App=()=>{element};export {{App}};"
    );
    // When: the last importer composes styles directly against the middle module's real pair.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", middle),
            (filename, &app),
        ],
        &format!(
            "const raw=modules[{filename:?}].App();const final=render(raw);JSON.stringify([slots(raw).length,final.target,final.props['data-tone'],final.props.ref,final.props.children,values(final,'color'),values(final,'padding'),slots(final),identities(final).length,new Set(identities(final)).size,modules['/src/base.ts'].trace]);"
        ),
    );
    // Then: the custom renderer and A closure remain while later fixed CSS wins exactly once.
    assert_eq!(
        result,
        serde_json::json!([
            1,
            "aside",
            "caller",
            7,
            "child",
            ["blue"],
            ["4px"],
            [],
            2,
            2,
            ["controller", "leaf"]
        ])
    );
}

#[rstest]
#[case("/src/app.ts")]
#[case("/src/app.tsx")]
#[serial]
fn bound_import_keeps_original_value_when_lowercase_name_requires_eager_binding(
    #[case] filename: &str,
) {
    // Given: a lowercase imported value uses the canonical eager bound-target path.
    let app = "import {styled} from '@devup-ui/react';import {Base as base} from './base';const Card=styled(base)({color:'blue'});export {Card};";
    // When: real emitted code renders the retained lowercase binding twice.
    let result = run(
        &[("/src/base.ts", BASE), (filename, app)],
        &format!(
            "const Card=modules[{filename:?}].Card;const before=modules['/src/base.ts'].trace.slice();const first=render(jsx(Card,{{ref:7,children:'first'}}));const second=render(jsx(Card,{{ref:8,children:'second'}}));JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children,values(e,'color'),slots(e)]),modules['/src/base.ts'].trace]);"
        ),
    );
    // Then: the renderer is a value, never a lowercase native tag or an invalid JSX target.
    assert_eq!(
        result,
        serde_json::json!([
            [],
            [
                ["article", 7, "first", ["blue"], []],
                ["article", 8, "second", ["blue"], []]
            ],
            ["controller", "leaf", "controller", "leaf"]
        ])
    );
}

#[rstest]
#[case::ordinary(("styled(Base)({opacity:p=>(trace.push('guard'),enabled)?({a:0.5,b:{nested:'raw'}})[state.key]:undefined,...(enabled?{lineHeight:[({a:1.5,b:1.7})[state.key],null,2.5],_hover:({a:{color:['red',null,'orange']},b:{color:['blue',null,'purple']}})[state.key]}:{}),p:p=>(trace.push('spacing'),2),margin:p=>(trace.push('pixels'),2)})", "2"))]
#[case::whole(("styled(Base)(p=>({...((trace.push('guard'),enabled)?{opacity:({a:0.5,b:{nested:'raw'}})[state.key],lineHeight:[({a:1.5,b:1.7})[state.key],null,2.5],_hover:({a:{color:['red',null,'orange']},b:{color:['blue',null,'purple']}})[state.key]}:{}),p:(trace.push('spacing'),2),margin:(trace.push('pixels'),2)}))", "2px"))]
#[case::template(("styled(Base)`${p=>({...((trace.push('guard'),enabled)?{opacity:({a:0.5,b:{nested:'raw'}})[state.key],lineHeight:[({a:1.5,b:1.7})[state.key],null,2.5],_hover:({a:{color:['red',null,'orange']},b:{color:['blue',null,'purple']}})[state.key]}:{}),p:(trace.push('spacing'),2),margin:(trace.push('pixels'),2)})}`", "2px"))]
#[serial]
fn scalar_and_rule_lookups_when_staged_keep_native_selection_and_property_units(
    #[case] fixture: (&str, &str),
    #[values(false, true)] single_css: bool,
) {
    // Given: each route extends a real retained base and uses independent native key coercion.
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Base}} from './base';const trace=[];let enabled,key;const state={{get key(){{trace.push('key');return {{toString(){{trace.push('coerce');if(key==='throw')throw new Error('stop');return key;}}}};}}}};const Card={};const set=(on,next)=>{{enabled=on;key=next;trace.length=0;}};export {{Card,trace,set}};",
        fixture.0
    );
    let base = BASE.replace(
        "color:'green'",
        "color:'green',lineHeight:[null,1.25],_hover:{color:[null,'yellow']}",
    );
    let observation = r"
const app=modules['/src/app.ts'];const before=[app.trace.slice(),modules['/src/base.ts'].trace.slice()];
const scenarios=[[false,'b'],[true,'a'],[true,'b'],[true,'missing'],[true,'throw']];
const normalize=values=>values.map(value=>typeof value==='number'?String(value):value);
const expected=scenarios.map(([enabled,key])=>{const trace=[];const state={get key(){trace.push('key');return {toString(){trace.push('coerce');if(key==='throw')throw new Error('stop');return key;}};}};
try{const rules={...((trace.push('guard'),enabled)?{opacity:({a:0.5,b:{nested:'raw'}})[state.key],lineHeight:[({a:1.5,b:1.7})[state.key],null,2.5],_hover:({a:{color:['red',null,'orange']},b:{color:['blue',null,'purple']}})[state.key]}:{}),p:(trace.push('spacing'),2),margin:(trace.push('pixels'),2)};
const responsive=[0,1,2].map(level=>[normalize([rules.lineHeight?.[level]??[null,1.25][level]].filter(v=>v!=null)),[rules._hover?.color[level]??[null,'yellow'][level]].filter(v=>v!=null)]);
return [[rules.opacity==null?[]:normalize([rules.opacity]),rules._hover?[rules._hover.color[0]]:[],['8px'],[LONGHAND],['green'],responsive],trace];}catch(error){return [error.message,trace];}});
const actual=scenarios.map(([on,key])=>{app.set(on,key);try{const e=render(jsx(app.Card,{}));return [[normalize(values(e,'opacity')),values(e,'color',0,'&:hover'),normalize(values(e,'padding')),normalize(values(e,'margin')),values(e,'color'),[0,1,2].map(level=>[normalize(values(e,'line-height',level)),values(e,'color',level,'&:hover')])],app.trace.slice()];}catch(error){return [error.message,app.trace.slice()];}});
JSON.stringify([before,actual,expected,modules['/src/base.ts'].trace]);
".replace("LONGHAND", &format!("{:?}", fixture.1));
    // When: hit, container, miss, throw and inactive cases traverse the actual staged renderers.
    let result = run_with_options(
        &[("/src/base.ts", &base), ("/src/app.ts", &app)],
        &observation,
        RunOptions {
            readable: true,
            single_css,
        },
    );
    // Then: all active declarations and effects agree with authored evaluation, including throw cutoff.
    assert_eq!(result[0], serde_json::json!([[], []]));
    assert_eq!(result[1], result[2], "{result}");
    assert_eq!(
        result[3],
        serde_json::json!([
            "controller",
            "leaf",
            "controller",
            "leaf",
            "controller",
            "leaf",
            "controller",
            "leaf"
        ])
    );
}

#[rstest]
#[case::jsx(("import {jsx as build} from 'react/jsx-runtime';", "build(ui.Base, (PROPS))", false))]
#[case::jsxs(("import {jsxs as build} from 'react/jsx-runtime';", "build(ui.Base, ((PROPS as const)))", false))]
#[case::jsx_dev(("import {jsxDEV as build} from 'react/jsx-dev-runtime';", "build(ui.Base, (PROPS satisfies object))", false))]
#[case::create_element(("import * as React from 'react';", "React.createElement(ui.Base, ((PROPS) as const), (order.push('children'),'child'))", true))]
#[serial]
fn wrapped_factory_props_when_retained_keep_css_slots_forwarding_and_native_order(
    #[case] fixture: (&str, &str, bool),
    #[values(false, true)] readable: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: a namespace retained definition owns attrs while each caller owns its props and ref.
    let base = BASE.replace(
        "styled.article(",
        "styled.article.attrs({'data-base':'attrs'})(",
    );
    let children = if fixture.2 {
        ""
    } else {
        ",children:(order.push('children'),'child')"
    };
    let props = format!(
        "{{title:(order.push('props'),'caller'),as:(order.push('as'),target),css:{{padding:'4px'}},ref:7,className:'caller',style:{{color:'inline'}}{children}}}"
    );
    let call = fixture.1.replace("PROPS", &props);
    let app = format!(
        "/** @jsxImportSource @emotion/react */\n{}import * as ui from './base';const App=target=>{call};export {{App}};",
        fixture.0
    );
    // When: real factory execution, including the original receiver, precedes retained rendering.
    let result = run_with_options(
        &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
        "JSON.stringify(['nav','section'].map(target=>{const raw=modules['/src/app.tsx'].App(target);const e=render(raw);return [e.target,e.props.title,e.props['data-base'],e.props.ref,e.props.children,e.props.className.split(/\\s+/).includes('caller'),e.props.style.color,values(e,'color'),values(e,'padding'),slots(e),order.slice(),modules['/src/base.ts'].trace.slice(),raw.props.css?.padding??null];}));",
        RunOptions {
            readable,
            single_css,
        },
    );
    // Then: private slot transport does not change CSS ownership, DOM forwarding or source effects.
    let effects = if fixture.2 {
        vec!["callee", "props", "as", "children", "receiver"]
    } else {
        vec!["props", "as", "children"]
    };
    // Plain React.createElement keeps user CSS even under an Emotion JSX pragma.
    for (index, target) in ["nav", "section"].iter().enumerate() {
        assert_eq!(
            result[index],
            serde_json::json!([
                target,
                "caller",
                "attrs",
                7,
                "child",
                true,
                "inline",
                ["green"],
                if fixture.2 { vec![] } else { vec!["4px"] },
                [],
                effects.repeat(index + 1),
                (["controller", "leaf"].repeat(index + 1)),
                if fixture.2 { Some("4px") } else { None }
            ])
        );
    }
}

#[rstest]
#[serial]
fn unbranded_react_namespace_when_opaque_keeps_user_css(
    #[values(false, true)] readable: bool,
    #[values(false, true)] single_css: bool,
) {
    let app = "import {jsx} from 'react/jsx-runtime';import * as ui from './base';const App=()=>jsx(ui.Base,{css:{padding:'4px'}});export {App};";
    let result = run_with_options(
        &[("/src/base.ts", BASE), ("/src/app.tsx", app)],
        "const raw=modules['/src/app.tsx'].App();const e=render(raw);JSON.stringify([raw.props.css.padding,values(e,'padding')]);",
        RunOptions {
            readable,
            single_css,
        },
    );
    assert_eq!(result, serde_json::json!(["4px", []]));
}
