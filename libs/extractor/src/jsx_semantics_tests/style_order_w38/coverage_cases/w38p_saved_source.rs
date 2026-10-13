use super::w38o_logical_source::Observation;
use super::*;

pub(super) struct Fixture<'a> {
    pub left: &'a str,
    pub selection: &'a str,
    pub consumer: &'a str,
}

pub(super) const OR: Fixture<'static> = Fixture {
    left: "const left=makeCss(mark('left',state.l)?{styleOrder:2,color:'red'}:null);",
    selection: "left||right",
    consumer: "css(picked,{styleOrder:1,background:'black'})",
};
pub(super) const AND: Fixture<'static> = Fixture {
    selection: "left&&right",
    ..OR
};
pub(super) const COALESCE: Fixture<'static> = Fixture {
    selection: "left??right",
    ..OR
};
pub(super) const CONDITIONAL: Fixture<'static> = Fixture {
    selection: "mark('choose',state.f)?left:right",
    ..OR
};
pub(super) const NONEMPTY_OR: Fixture<'static> = Fixture {
    left: "const left=makeCss({styleOrder:2,color:'red'});",
    ..OR
};
pub(super) const EMPTY_AND: Fixture<'static> = Fixture {
    left: "const left=makeCss();",
    ..AND
};
pub(super) const LAZY_OR: Fixture<'static> = Fixture {
    consumer: "css(picked||{styleOrder:1,background:mark('fallback',b)?'black':'white'})",
    ..OR
};

pub(super) fn source(input: &Fixture<'_>) -> String {
    let Fixture {
        left,
        selection,
        consumer,
    } = input;
    format!(
        "import {{css as makeCss}} from '@devup-ui/react';\n\
         import {{ClassNames}} from '@emotion/react';\n\
         const render=(l,r,f,b)=>{{const mark=(n,v)=>(trace.push(n),v);\
         const state={{l,r,f}};{left}\
         const right=makeCss(mark('right',state.r)?{{styleOrder:3,color:'blue'}}:null);\
         const selected={selection};state.l=!state.l;state.r=!state.r;state.f=!state.f;\
         const parts=[selected];const alias=parts;const picked=alias['0'];\
         return <ClassNames>{{({{css}})=>{consumer}}}</ClassNames>}};"
    )
}

pub(super) fn observe(input: &Fixture<'_>, invocation: &str) -> Observation {
    let output = compile_emotion(&source(input)).required("saved finite ClassNames must compile");
    let evaluated = whole::evaluate_code(&output.code, invocation);
    Observation { output, evaluated }
}
