pub(super) fn files(local: bool, hybrid: bool) -> Vec<(String, String)> {
    let base = "import {styled} from '@devup-ui/react';const trace=[];export const observe=()=>trace;export const Dyn=styled.button.attrs({id:'base'})(p=>p.on?{color:'red'}:{});";
    let attrs = if hybrid {
        "p=>(trace.push('attrs',p.style===undefined),{})"
    } else {
        "{title:'plain'}"
    };
    let extension = format!(
        "const W=Dyn.withComponent('a');const Card=styled(W).attrs({attrs})({{}});export const App=style=>jsx(Card,{{on:true,style,className:'external',ref:7,children:'child'}});"
    );
    if local {
        vec![(
            "/identity/local.js".into(),
            format!("{base}import {{jsx}} from '@emotion/react';{extension}"),
        )]
    } else {
        vec![
            ("/identity/base.js".into(), base.into()),
            (
                "/identity/entry.js".into(),
                format!(
                    "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{Dyn,observe}} from './base.js';const trace=observe();{extension}"
                ),
            ),
        ]
    }
}

pub(super) fn observation(local: bool, provided: bool) -> String {
    let entry = if local { "local" } else { "entry" };
    let origin = if local { "local" } else { "base" };
    let style = if provided {
        "{get ['--caller'](){trace.push('style');return 'gold';}}"
    } else {
        "undefined"
    };
    format!(
        "import {{App}} from '/identity/{entry}.js';import {{observe}} from '/identity/{origin}.js';const trace=observe();const style={style};const e=render(App(style));globalThis.observed={{sameStyle:e.props.style===style,absentStyle:e.props.style===undefined,trace:trace.slice(),target:e.target,props:e.props}};"
    )
}

pub(super) fn empty() -> (Vec<(String, String)>, String) {
    (vec![("/identity/empty.js".into(), "import {styled} from '@devup-ui/react';import {jsx} from '@emotion/react';const Target=p=>({target:'custom',props:p});const Card=styled(Target).attrs({id:'base'})({});export const App=()=>jsx(Card,{ref:7,children:'child'});".into())],
    "import {App} from '/identity/empty.js';const e=render(App());globalThis.observed={classType:typeof e.props.className,styleType:typeof e.props.style,target:e.target,props:e.props};".into())
}

pub(super) fn targets(mode: &str) -> (Vec<(String, String)>, String) {
    let base = format!(
        "import {{styled}} from '@devup-ui/react';const trace=[];export const observe=()=>trace;const targets={{get Card(){{trace.push('origin-default');{}return 'button';}}}};export const Dyn=styled(targets.Card)((p,unused=(trace.push('callback'),0))=>p.on?{{color:'red'}}:{{}});",
        if mode == "throw" {
            "throw new RangeError('target');"
        } else {
            ""
        }
    );
    let first = if mode == "origin" || mode == "throw" {
        "styled(Dyn)({})"
    } else {
        "Dyn.withComponent(targets.Card)"
    };
    let first = format!(
        "import {{styled}} from '@devup-ui/react';import {{Dyn,observe}} from './base.js';const trace=observe();const targets={{get Card(){{trace.push('first-default');return 'a';}}}};export const First={first};"
    );
    let target = if mode == "repeat" {
        "First.withComponent(targets.Card)"
    } else {
        "First"
    };
    let as_prop = if mode == "caller" {
        "'nav'"
    } else {
        "undefined"
    };
    let entry = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{First}} from './first.js';import {{observe}} from './base.js';const trace=observe();const targets={{get Card(){{trace.push('last-default');return 'section';}}}};const Card={target};export const App=()=>jsx(Card,{{on:true,as:{as_prop},ref:7,children:'child'}});"
    );
    let observer = "import {App} from '/targets/entry.js';import {observe} from '/targets/base.js';const before=observe().slice();try{const e=render(App());globalThis.observed={before,trace:observe().slice(),target:e.target,props:e.props};}catch(error){globalThis.observed={before,trace:observe().slice(),error:error.name,props:{}};}";
    (
        vec![
            ("/targets/base.js".into(), base),
            ("/targets/first.js".into(), first),
            ("/targets/entry.js".into(), entry),
        ],
        observer.into(),
    )
}

pub(super) fn callback_frame() -> (Vec<(String, String)>, String) {
    (vec![
        ("/callback-frame/base.js".into(), "import {styled} from '@devup-ui/react';const trace=[];export const observe=()=>trace;export const Dyn=styled.button(function(p,unused=(trace.push('origin'),p.changed='origin',0)){return p.on?{color:'red'}:{};});".into()),
        ("/callback-frame/entry.js".into(), "import {styled} from '@devup-ui/react';import {jsx} from '@emotion/react';import {Dyn,observe} from './base.js';const trace=observe();const W=Dyn.withComponent('a');const Card=styled(W)(function(p,unused=(trace.push('own',p.changed),0)){return p.on?{backgroundColor:'blue'}:{};});export const App=()=>jsx(Card,{on:true,ref:7,children:'child'});".into()),
    ], "import {App} from '/callback-frame/entry.js';import {observe} from '/callback-frame/base.js';const e=render(App());globalThis.observed={trace:observe(),target:e.target,props:e.props};".into())
}
