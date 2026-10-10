use super::*;
use crate::extract_style::style_property::StyleProperty;

pub(super) struct Fixture<'a> {
    pub outer_producer: &'a str,
    pub returned_expression: &'a str,
}

pub(super) const EMPTY_OR: Fixture<'static> = Fixture {
    outer_producer: "const saved=makeCss(mark('construct',active)?{styleOrder:2,color:'red'}:null);",
    returned_expression: "css(saved||{styleOrder:2,color:'blue'})",
};
pub(super) const EMPTY_COALESCE: Fixture<'static> = Fixture {
    outer_producer: EMPTY_OR.outer_producer,
    returned_expression: "css(saved??{styleOrder:2,color:'blue'})",
};
pub(super) const NONEMPTY_OR: Fixture<'static> = Fixture {
    outer_producer: concat!(
        "const saved=makeCss(mark('construct',active)?",
        "{styleOrder:2,color:'red'}:{styleOrder:2,color:'green'});"
    ),
    returned_expression: "css(saved||{styleOrder:2,color:'blue'})",
};
pub(super) const NESTED_GUARDS: Fixture<'static> = Fixture {
    outer_producer: "",
    returned_expression: concat!(
        "css({styleOrder:2,color:'black'},mark('outer',outer)&&",
        "(mark('inner',inner)?{styleOrder:2,color:'red'}:{styleOrder:2,color:'green'}))"
    ),
};

pub(super) struct Observation {
    pub output: ExtractOutput,
    pub evaluated: whole::Evaluation,
}

struct DebugScope(bool);

impl Drop for DebugScope {
    fn drop(&mut self) {
        css::debug::set_debug(self.0);
    }
}

pub(super) fn fixture(input: &Fixture<'_>) -> String {
    let Fixture {
        outer_producer,
        returned_expression,
    } = input;
    format!(
        "import {{ClassNames}} from '@emotion/react';import {{css as makeCss}} from '@devup-ui/react';\nconst render=(active,outer,inner)=>{{const mark=(n,v)=>(trace.push(n),v);{outer_producer}return <ClassNames>{{({{css}})=>{returned_expression}}}</ClassNames>}};"
    )
}

pub(super) fn observe(input: &Fixture<'_>, invocation: &str) -> Observation {
    let output = compile_emotion(&fixture(input)).required("logical ClassNames must compile");
    let evaluated = whole::evaluate_code(&output.code, invocation);
    Observation { output, evaluated }
}

pub(super) fn declarations(
    styles: &[ExtractStyleValue],
    tokens: &[&str],
) -> Vec<ExtractStyleValue> {
    let _debug = DebugScope(css::debug::is_debug());
    css::debug::set_debug(true);
    let mut selected: Vec<_> = styles
        .iter()
        .filter(|style| {
            style
                .extract(Some("a.tsx"))
                .is_some_and(|property| match property {
                    StyleProperty::ClassName(name)
                    | StyleProperty::Variable {
                        class_name: name, ..
                    } => tokens.contains(&name.as_str()),
                })
        })
        .cloned()
        .collect();
    selected.sort_unstable();
    selected
}
