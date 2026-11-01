use crate::{
    extractor::selector_params::{ArgumentList, parse},
    own_css_plan::{Node, ParamEntry, Plan},
};

#[derive(Debug, Clone)]
pub(crate) enum Scalar {
    Text(String),
    Absent,
    Invalid,
}

pub(crate) struct Params<'p> {
    pub plan: &'p Plan,
    pub value: usize,
}

impl Params<'_> {
    pub(crate) fn resolve(
        &self,
        mut scalar: impl FnMut(usize) -> Scalar,
        required: impl FnOnce() -> bool,
    ) -> Result<Option<String>, ()> {
        let Self { plan, value } = *self;
        let list = match &plan.nodes[value] {
            Node::Array(items) => ArgumentList::List(
                items.iter().map(|&id| match plan.param_entry(id) {
                    ParamEntry::Hole => Ok(None),
                    ParamEntry::Invalid => Err(()),
                    ParamEntry::Scalar(slot) => match scalar(slot) {
                        Scalar::Text(text) => Ok(Some((text, ()))),
                        Scalar::Absent | Scalar::Invalid => Err(()),
                    },
                }),
                (),
            ),
            Node::Scalar(slot) if matches!(scalar(*slot), Scalar::Absent) => {
                ArgumentList::Absent(())
            }
            Node::Scalar(_) | Node::Object(_) => ArgumentList::Invalid(()),
        };
        parse(list, required)
    }
}

pub(crate) fn literal(expression: &oxc_ast::ast::Expression<'_>) -> Scalar {
    if crate::extractor::style_inputs::absent(expression, None) {
        Scalar::Absent
    } else {
        crate::utils::get_string_by_literal_expression(expression)
            .map_or(Scalar::Invalid, |text| Scalar::Text(text.into_owned()))
    }
}
