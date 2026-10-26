pub(super) fn files(surface: &str, target: &str, jsx: bool) -> Vec<(String, String)> {
    let base = r"import {styled} from '@devup-ui/react';
const trace=[];
const baseStamp=name=>(trace.push(name),0);
export const observe=()=>trace;
export const Target=p=>(trace.push('target',p.as),{target:'custom',props:p});
export const Dyn=styled.button.attrs(p=>(baseStamp('base-attrs'),{order:'base',className:'baseAttr',style:{baseStyle:1}}))(
function(p,unused=(baseStamp('base-default'),trace.push(this===undefined,arguments.length,'base-callback',p.order),0)){
 return p.pick?{color:'red',fontSize:['20px','24px'],_hover:{color:'maroon'},'@layer':{base:{color:'coral'}}}:{color:'green',fontSize:['10px','14px'],_hover:{color:'olive'},'@layer':{base:{color:'orange'}}};
});";
    let (import, receiver) = match surface {
        "named" => ("import {Dyn} from './base.js';", "Dyn"),
        "reserved" => (
            "import {Dyn as __devupRecord1} from './base.js';",
            "__devupRecord1",
        ),
        "reexport" => ("import {Renamed as Dyn} from './forward.js';", "Dyn"),
        "dot" => ("import * as UI from './forward.js';", "UI.Renamed"),
        "computed" => ("import * as UI from './forward.js';", "UI['Renamed']"),
        _ => panic!("unknown import surface"),
    };
    let first = format!(
        r"import {{styled}} from '@devup-ui/react';{import}import {{observe}} from './base.js';
const trace=observe();
const Seed={receiver}.withComponent('a');
const ownStamp=name=>(trace.push(name),0);
export const First=styled(Seed).attrs(p=>(ownStamp('own-attrs'),{{order:'own',className:'ownAttr',style:{{ownStyle:2}}}}))(
function(p,unused=(ownStamp('own-default'),trace.push(this===undefined,arguments.length,'own-callback',p.order),0)){{
return p.replace?{{color:'blue',fontSize:[null,'8px'],_hover:{{color:'navy'}},'@layer':{{base:{{color:'cyan'}}}}}}:{{}};
}}); "
    );
    let retarget = match target {
        "native" => "'section'",
        "component" | "as" => "Target",
        _ => panic!("unknown target"),
    };
    let call = if jsx {
        "<Card pick={pick} replace={replace} last={last} ref={7} children='child' className='external' style={{caller:3}} forwardedAs='em' as={as} $private='secret' blocked='value'/>"
    } else {
        "jsx(Card,{pick,replace,last,ref:7,children:'child',className:'external',style:{ caller:3 },forwardedAs:'em',as,$private:'secret',blocked:'value'})"
    };
    let entry = format!(
        r"import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{First}} from './first.js';import {{Target,observe}} from './base.js';
const trace=observe();
const Again=First.withComponent({retarget});
const Last=styled(Again).attrs(p=>(trace.push('last-attrs'),{{order:'last',className:'lastAttr'}}))((p,unused=(trace.push('last-callback',p.order),0))=>p.last?{{fontSize:'7px'}}:{{}});
const Card=Last.withComponent({retarget});
export const App=(pick,replace,last,as)=>{call};"
    );
    vec![
        ("/stages/base.js".into(), base.into()),
        (
            "/stages/forward.js".into(),
            "export {Dyn as Renamed} from './base.js';".into(),
        ),
        ("/stages/first.js".into(), first),
        (
            format!("/stages/entry.{}", if jsx { "jsx" } else { "js" }),
            entry,
        ),
    ]
}

pub(super) fn observation(jsx: bool, selected: bool, target: &str) -> String {
    observation_states(jsx, target, [selected; 3])
}

pub(super) fn local(jsx: bool) -> Vec<(String, String)> {
    let files = files("named", "native", jsx);
    let own = files[2]
        .1
        .split_once("const Seed=")
        .unwrap_or_else(|| panic!("own stage source"))
        .1;
    let last = files[3]
        .1
        .split_once("const Again=")
        .unwrap_or_else(|| panic!("last stage source"))
        .1;
    vec![(
        format!("/stages/local.{}", if jsx { "jsx" } else { "js" }),
        format!(
            "{}import {{jsx}} from '@emotion/react';const Seed={own}const Again={last}",
            files[0].1
        ),
    )]
}

pub(super) fn exported_retarget() -> Vec<(String, String)> {
    let mut files = files("named", "native", false);
    files[0].1 = files[0].1.replacen("export const Dyn=", "const Base=", 1);
    files[0]
        .1
        .push_str("export const Dyn=Base.withComponent('button');");
    files
}

pub(super) fn local_observation(jsx: bool, selected: bool) -> String {
    let path = format!("/stages/local.{}", if jsx { "jsx" } else { "js" });
    observation(jsx, selected, "native")
        .replace("/stages/entry.jsx", &path)
        .replace("/stages/entry.js", &path)
        .replace("/stages/base.js", &path)
}

pub(super) fn observation_states(jsx: bool, target: &str, states: [bool; 3]) -> String {
    let override_as = if target == "as" { "'nav'" } else { "undefined" };
    let [pick, replace, last] = states;
    format!(
        r"import {{App}} from '/stages/entry.{}';import {{observe}} from '/stages/base.js';
const e=render(App({pick},{replace},{last},{override_as}));
globalThis.observed={{trace:observe(),target:e.target,props:e.props}};",
        if jsx { "jsx" } else { "js" }
    )
}

pub(super) fn callbacks(local: bool) -> Vec<(String, String)> {
    let base = r"import {styled} from '@devup-ui/react';
const trace=[];export const observe=()=>trace;
export const Dyn=styled.button({
color:function(p,unused=(trace.push('normal-default',this===undefined,arguments.length),0)){return p.input.color;},
margin:(p,unused=(trace.push('lexical-default',this===undefined),0))=>p.input.margin
});";
    let extension = "const Seed=Dyn.withComponent('a');const Card=styled(Seed)({color:'blue'});export const App=input=>jsx(Card,{input,ref:7,children:'child'});";
    if local {
        vec![(
            "/callbacks/local.js".into(),
            format!("{base}import {{jsx}} from '@emotion/react';{extension}"),
        )]
    } else {
        vec![
            ("/callbacks/base.js".into(), base.into()),
            (
                "/callbacks/entry.js".into(),
                format!(
                    "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{Dyn}} from './base.js';{extension}"
                ),
            ),
        ]
    }
}

pub(super) fn callback_observation(local: bool) -> String {
    let entry = if local { "local" } else { "entry" };
    let origin = if local { "local" } else { "base" };
    format!(
        r"import {{App}} from '/callbacks/{entry}.js';import {{observe}} from '/callbacks/{origin}.js';
const trace=observe();const input={{get color(){{trace.push('color',this===input);return 'red';}},get margin(){{trace.push('margin',this===input);return '6px';}}}};
const e=render(App(input));globalThis.observed={{target:e.target,props:e.props,trace}};"
    )
}

pub(super) fn dynamic_css() -> (Vec<(String, String)>, String) {
    let files = vec![
        ("/dynamic-css/base.js".into(), "import {styled} from '@devup-ui/react';export const Dyn=styled.button`font-size:${p=>p.big?'20px':'10px'};`;".into()),
        ("/dynamic-css/entry.js".into(), "import {styled} from '@devup-ui/react';import {jsx} from '@emotion/react';import {Dyn} from './base.js';export const W=Dyn.withComponent('a');export const App=()=>jsx(W,{css:{backgroundColor:'blue'},big:true,ref:7,children:'child'});".into()),
    ];
    (files, "import {App,W} from '/dynamic-css/entry.js';const raw=App();const e=render(raw);globalThis.observed={rawIdentity:raw.target===W,rawSlots:slots(raw).length,target:e.target,props:e.props};".into())
}
