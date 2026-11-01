#[derive(Clone, Copy, Debug)]
pub(super) enum Site {
    LocalJsx,
    LocalCompiled,
    NamespaceJsx,
    NamespaceCompiled,
    NamespaceComputed,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Inline,
    Configured,
    Slot,
    Retained,
    Primitive,
    Generic,
    Effects,
    Accessors,
    RetainedAccessors,
}

pub(super) struct Fixture {
    pub files: Vec<(String, String)>,
    pub observation: String,
}

pub(super) fn fixture(site: Site, case: Case) -> Fixture {
    let namespace = matches!(
        site,
        Site::NamespaceJsx | Site::NamespaceCompiled | Site::NamespaceComputed
    );
    let jsx = matches!(site, Site::LocalJsx | Site::NamespaceJsx);
    let definition = match case {
        Case::Inline => "export const Card=styled.article({color:'green',padding:'2px'});",
        Case::Configured | Case::Effects | Case::Accessors => {
            "export const Card=styled.article.withConfig({shouldForwardProp:k=>k!=='blocked'})({color:'green',padding:'2px'});"
        }
        Case::Slot => {
            "export const Card=styled.article({...((baseTrace.push('base'),true)?{color:'red'}:{}),color:'green',padding:'2px'});"
        }
        Case::Retained | Case::RetainedAccessors => {
            "export const Card=styled.article.withConfig({shouldForwardProp:k=>k!=='blocked'})({color:p=>p.tone,padding:'2px'});"
        }
        Case::Primitive => "export const Card='article';",
        Case::Generic => "export const Card=props=>jsx('article',props);",
    };
    let header = "import {styled} from '@devup-ui/react';import {jsx,} from '@emotion/react';";
    let base =
        format!("{header}const baseTrace=[];{definition}export const baseObserve=()=>baseTrace;");
    let scope = if namespace {
        "import * as UI from './base.js';export {baseObserve} from './base.js';".into()
    } else {
        format!("const baseTrace=[];{definition}export const baseObserve=()=>baseTrace;")
    };
    let target = match site {
        Site::LocalJsx | Site::LocalCompiled => "Card",
        Site::NamespaceJsx | Site::NamespaceCompiled => "UI.Card",
        Site::NamespaceComputed => "UI['Card']",
    };
    let css = if matches!(case, Case::Retained | Case::RetainedAccessors) {
        "backgroundColor:'blue'"
    } else {
        "color:'blue'"
    };
    let private = if matches!(case, Case::Effects) {
        "input.private"
    } else {
        "'hide'"
    };
    let blocked = if matches!(case, Case::Effects) {
        "dropped()"
    } else {
        "'hide'"
    };
    let before = if matches!(case, Case::Effects) {
        "mark('before','first')"
    } else {
        "'first'"
    };
    let after = if matches!(case, Case::Effects) {
        "mark('after','last')"
    } else {
        "'last'"
    };
    let reference = if matches!(case, Case::Effects) {
        "mark('ref',7)"
    } else {
        "7"
    };
    let children = if matches!(case, Case::Effects) {
        "mark('children','child')"
    } else {
        "'child'"
    };
    let call = if jsx {
        format!(
            "<{target} css={{{{{css}}}}} data-first={{{before}}} $private={{{private}}} blocked={{{blocked}}} onClick={{handler}} ref={{{reference}}} children={{{children}}} data-last={{{after}}} aria-label='label' className='external' style={{userStyle}} tone='purple' />"
        )
    } else {
        let denied = if matches!(case, Case::Accessors | Case::RetainedAccessors) {
            "get $private(){trace.push('private',this['data-first']);return 'hide';},get blocked(){trace.push('blocked',this['data-first']);return 'hide';},".into()
        } else {
            format!("$private:{private},blocked:{blocked},")
        };
        format!(
            "jsx({target},{{css:{{{css}}},'data-first':{before},{denied}onClick:handler,ref:{reference},children:{children},'data-last':{after},'aria-label':'label',className:'external',style:userStyle,tone:'purple'}})"
        )
    };
    let entry = format!("/boundary/entry.{}", if jsx { "jsx" } else { "js" });
    let code = format!(
        "{header}{scope}const trace=[];const input={{get private(){{trace.push('getter');return 'hide';}}}};const mark=(name,value)=>(trace.push(name),value);function dropped(value=(trace.push('default'),0)){{trace.push(this===undefined?'this':'receiver',arguments.length);return 'hide';}}export const handler=()=>0;const userStyle={{borderColor:'black'}};export const observe=()=>trace;export const App=()=>{call};"
    );
    let mut files = Vec::new();
    if namespace {
        files.push(("/boundary/base.js".into(), base));
    }
    files.push((entry.clone(), code));
    Fixture {
        files,
        observation: format!(
            "import {{App,handler,observe,baseObserve}} from '{entry}';const raw=App();const final=render(raw);globalThis.observed={{rawNative:typeof raw.target==='string',rawSlots:slots(raw).length,private:Object.hasOwn(final.props,'$private'),blocked:Object.hasOwn(final.props,'blocked'),trace:observe(),baseTrace:baseObserve(),public:[final.target,final.props['data-first'],final.props['data-last'],final.props['aria-label'],final.props.onClick===handler,final.props.ref,final.props.children,final.props.style.borderColor,final.props.className.includes('external')],classes:final.props.className.split(/\\s+/),style:final.props.style||{{}}}};"
        ),
    }
}
