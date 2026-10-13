use super::w38o_logical_source::Observation;
use super::*;

pub(super) struct Fixture<'a> {
    pub leaf: &'a str,
    pub saved: bool,
}

pub(super) const OBJECT: &str = "const object={get value(){trace.push('getter');return external}};";
pub(super) const PRODUCER: &str =
    "const saved=makeCss(mark('construct',active)?{styleOrder:1,background:'black'}:null);";

pub(super) fn fixture(input: &Fixture<'_>) -> String {
    let producer = if input.saved { PRODUCER } else { "" };
    let saved = if input.saved { ",saved" } else { "" };
    format!(
        "import {{ClassNames}} from '@emotion/react';import {{css as makeCss}} from '@devup-ui/react';const render=(external,active)=>{{const mark=(n,v)=>(trace.push(n),v);{OBJECT}{producer}return <ClassNames>{{({{css,cx}})=>cx(css({{styleOrder:2,color:'red'}}){saved},{})}}</ClassNames>}};",
        input.leaf,
    )
}

struct DebugScope(bool);
impl Drop for DebugScope {
    fn drop(&mut self) {
        css::debug::set_debug(self.0);
    }
}

pub(super) fn observe(input: &Fixture<'_>, external: &str) -> Observation {
    let _debug = DebugScope(css::debug::is_debug());
    let output = compile_emotion(&fixture(input)).required("external leaf must compile");
    let invocation = format!("render({},true)", serde_json::json!(external));
    let evaluated = whole::evaluate_code(&output.code, &invocation);
    Observation { output, evaluated }
}

pub(super) fn reference(saved: bool) -> ExtractOutput {
    let _debug = DebugScope(css::debug::is_debug());
    let source = if saved {
        "import {css} from '@devup-ui/react';const reference=[css({styleOrder:2,color:'red'}),css({styleOrder:1,background:'black'})];"
    } else {
        "import {css} from '@devup-ui/react';const reference=[css({styleOrder:2,color:'red'})];"
    };
    compile_emotion(source).required("independent literal array must compile without rendering")
}
