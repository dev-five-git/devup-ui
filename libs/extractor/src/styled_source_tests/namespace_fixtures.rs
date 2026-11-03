pub(super) fn fixture(kind: &str, surface: &str, selected: bool) -> Vec<(String, String)> {
    let rules = match kind {
        "inline" => "{color:'green',padding:'2px'}",
        "slot" | "inherited" | "retarget" | "forwarding" | "as" => {
            "{...((trace.push('controller'),selected)?{color:(trace.push('leaf'),'red')}:{ }),color:'green',padding:'2px'}"
        }
        "retained" => {
            "{color:function(p,unused=(trace.push('default',this===undefined,arguments.length),0)){return p.tone;},padding:'2px'}"
        }
        "choice" => {
            "{color:'green',margin:'1px',padding:'2px'},function(p,unused=(trace.push('default',this===undefined,arguments.length),0)){return selected?{margin:'4px'}:{}}"
        }
        _ => panic!("unknown fixture kind"),
    };
    let definition = match kind {
        "inherited" => format!(
            "const Base=styled.article({rules});export const Card=styled(Base)({{padding:'3px'}});"
        ),
        "retarget" => format!(
            "const Base=styled.article({rules});export const Card=Base.withComponent('section');"
        ),
        "forwarding" => format!(
            "export const Card=styled.article.withConfig({{shouldForwardProp:key=>key!=='blocked'}})({rules});"
        ),
        _ => format!("export const Card=styled.article({rules});"),
    };
    let base = format!(
        "import {{styled}} from '@devup-ui/react';const trace=[];let selected={selected};{definition}export const observe=()=>trace;export const toggle=()=>{{selected=!selected;}};"
    );
    let call = match surface {
        "jsx" => {
            "<UI.Card css={{color:'blue'}} ref={ref} children='child' className='external' tone='purple' blocked='hide' $private='hide' />"
        }
        "dot" => {
            "jsx(UI.Card,{css:{color:'blue'},ref,children:'child',className:'external',tone:'purple',blocked:'hide',$private:'hide'})"
        }
        "computed" => {
            "jsx(UI['Card'],{css:{color:'blue'},ref,children:'child',className:'external',tone:'purple',blocked:'hide',$private:'hide'})"
        }
        _ => panic!("unknown syntax"),
    };
    let call = match kind {
        "inline" => call
            .replace(" $private='hide'", "")
            .replace(",$private:'hide'", ""),
        "retained" | "choice" => call.replace("color:'blue'", "backgroundColor:'blue'"),
        "as" if surface == "jsx" => call
            .replace("color:'blue'", "backgroundColor:'blue'")
            .replace("ref={ref}", "as='nav' ref={ref}"),
        "as" => call
            .replace("color:'blue'", "backgroundColor:'blue'")
            .replace("ref,children", "as:'nav',ref,children"),
        _ => call.into(),
    };
    let entry = format!(
        "/** @jsxImportSource @emotion/react */\nimport {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import * as UI from './base.js';export {{Card as Alias}} from './base.js';export const Anchor=styled.div({{[UI.Card]:{{outlineColor:'orange'}}}});export const App=(trace,selected,ref)=>{call};"
    );
    vec![
        ("/namespace/base.js".into(), base),
        (
            format!(
                "/namespace/entry.{}",
                if surface == "jsx" { "jsx" } else { "js" }
            ),
            entry,
        ),
    ]
}

pub(super) fn observation(entry: &str) -> String {
    format!(
        r"import * as UI from './base.js';import {{App,Alias}} from './{entry}';
const before=UI.observe().slice();const raw=App([],false,7);const first=render(raw);UI.toggle();const second=render(App([],false,8));
globalThis.observed={{before,identity:Alias===UI.Card,rawIdentity:raw.target===UI.Card,rawSlots:slots(raw).length,trace:UI.observe(),elements:[first,second].map(e=>({{target:e.target,ref:e.props.ref,children:e.props.children,slots:slots(e),classes:e.props.className.split(/\s+/),style:e.props.style||{{}},transient:'$private' in e.props,blocked:'blocked' in e.props}}))}};"
    )
}
