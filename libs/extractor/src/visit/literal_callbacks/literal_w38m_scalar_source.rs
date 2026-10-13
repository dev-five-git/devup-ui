use super::literal_w38m_support::GETTERS;

const HEAD: &str = "import {styled} from '@devup-ui/react';const __devupForwardRef=render=>render;";
const ORDER: &str = "style-order:${p=>{trace.push('callback');return;}};";
pub(super) const EFFECTS: &str = "opacity:(trace.push('before'),1),_focus:{color:'green'},...{color:'red',zIndex:(trace.push('spread'),2),_active:{color:'yellow'}},flexGrow:(trace.push('after'),3),_disabled:{color:'gray'},_hover:`ORDERcolor:blue`";
pub(super) const PRECEDENCE: &str = "opacity:(trace.push('before'),1),_focus:{color:'green'},...{color:'red',zIndex:(trace.push('spread'),2),_active:{color:'yellow'}},color:'purple',order:(trace.push('between'),4),...{color:'red',zIndex:(trace.push('spread'),2),_active:{color:'yellow'}},color:'orange',flexGrow:(trace.push('after'),3),_disabled:{color:'gray'},_hover:`ORDERcolor:blue`";

pub(super) fn fixture(root: &str, bare: bool) -> String {
    let root = root.replace("ORDER", if bare { ORDER } else { "" });
    format!("{HEAD}const Card=styled('div',{{{root}}});{GETTERS}\n")
}

pub(super) fn throwing(bare: bool) -> String {
    let root = EFFECTS
        .replace("(trace.push('spread'),2)", "(opaque(),2)")
        .replace("ORDER", if bare { ORDER } else { "" });
    format!(
        "{HEAD}const sentinel={{}};const opaque=()=>{{trace.push('opaque');throw sentinel}};let caught;try{{const Card=styled('div',{{{root}}});trace.push('built');Card({{}},null);trace.push('after-render')}}catch(error){{if(error!==sentinel)throw error;caught=error}}const a={{props:{{className:caught===sentinel?'same':'different'}}}};const b={{props:{{className:'stopped'}}}};\n"
    )
}
