//! Strict final-value capture: no initializer fallback and no second native run.

use super::{
    BTreeSet, Binding, CollectedStyles, Context, FxHashMap, JsResult, JsValue, NameScope, Source,
    Stylesheet, execution, get_file_num_by_filename, name_values,
};
use crate::module_loader::Mapped;
mod emission;
mod graph;
mod mutations;
mod observations;
pub(crate) mod terminal;
pub(crate) use emission::Emission;
pub(super) use emission::Finished;
pub(super) use observations::Samplers;
pub(crate) use observations::{Observation, ObservationKind};

pub(crate) struct Capture {
    pub name: String,
    pub read: String,
    pub place: String,
    pub root: oxc_span::Span,
}

pub(crate) struct MutationCheck {
    pub read: String,
    pub path: Vec<Option<String>>,
    pub place: String,
}

pub(super) enum FinalizeError {
    Js(boa_engine::JsError),
    Located(String),
}

impl From<boa_engine::JsError> for FinalizeError {
    fn from(error: boa_engine::JsError) -> Self {
        Self::Js(error)
    }
}

pub(crate) struct Selected<'a> {
    pub stylesheet: Stylesheet<'a>,
    pub mapped: &'a Mapped,
    pub captures: &'a [Capture],
    pub reserved: &'a BTreeSet<String>,
    pub mutations: &'a [MutationCheck],
    pub observer: &'a str,
    pub observations: &'a [Observation],
}

pub(crate) fn execute(
    selected: Selected<'_>,
    loader: crate::module_loader::ModuleLoader<'_>,
) -> Result<super::execution::Executed, String> {
    execution::run(execution::Input::Selected(selected), loader)
}

pub(super) fn prepare(
    context: &mut Context,
    selected: &Selected<'_>,
    samplers: observations::Samplers,
) -> JsResult<()> {
    observations::prepare(context, selected, samplers)
}

pub(super) fn finish(
    selected: &Selected<'_>,
    collected: &mut CollectedStyles,
    context: &mut Context,
) -> Result<emission::Finished, FinalizeError> {
    let captures = selected.captures;
    let bindings: Vec<_> = captures
        .iter()
        .map(|capture| Binding {
            name: capture.name.clone(),
            exported: false,
            read: capture.read.clone(),
            alias: None,
            init: None,
        })
        .collect();
    let values: Vec<_> = bindings
        .iter()
        .map(|binding| {
            context
                .eval(Source::from_bytes(binding.read.as_bytes()))
                .map(|value| (binding, value))
        })
        .collect::<JsResult<_>>()?;
    mutations::check(selected.mutations, &values, context)?;
    let names = name_values(
        collected,
        &values,
        NameScope {
            file_num: get_file_num_by_filename(selected.stylesheet.filename),
            reserved: selected.reserved,
        },
    );
    let mut state = context
        .remove_data::<observations::State>()
        .ok_or_else(|| {
            FinalizeError::Located(format!(
                "{}:1:1: missing native capture observations. Fix: report this extraction error",
                selected.stylesheet.filename
            ))
        })?;
    if let Some(failure) = state.failure.take() {
        return Err(FinalizeError::Located(failure));
    }
    let roots = values.iter().zip(captures).map(|((_, value), capture)| {
        state.graph.value(value, context).map_err(|cause| FinalizeError::Located(format!(
            "{}: native result `{}` cannot be captured exactly: {cause}. Fix: return literals, arrays or plain records instead of functions, cyclic or exotic values",
            capture.place, capture.read)))
    }).collect::<Result<Vec<_>, _>>()?;
    emission::finish(
        &state,
        emission::Inputs {
            roots: &roots,
            captures,
            names: &names,
            reserved: selected.reserved,
        },
    )
}
