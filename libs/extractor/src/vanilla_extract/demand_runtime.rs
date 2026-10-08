use super::{
    Binding, Collector, NameScope, PACKAGE_BINDING, StyleCollector, StylesheetImports, name_values,
    register_vanilla_extract_apis,
};
use boa_engine::{
    Context, JsArgs, JsResult, JsString, JsValue, NativeFunction, property::Attribute,
};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

mod audit;
mod compile;
mod finalize;

struct Owner {
    plan: crate::module_loader::demand::Producer,
    collector: StyleCollector,
    finalized: bool,
    option: crate::ExtractOption,
}

struct State {
    owners: Vec<Owner>,
    audits: Vec<audit::Audit>,
    entry: StyleCollector,
    terminal: super::capture::terminal::Terminal,
    atoms: super::producer_atoms::ProducerAtoms,
    references: super::style_references::StyleReferences,
    artifacts: crate::graph::Artifacts,
}

pub(super) fn prepare(
    context: &mut Context,
    loader: &crate::module_loader::ModuleLoader<'_>,
    entry: &StyleCollector,
) -> Result<(), String> {
    let mut owners = Vec::new();
    for plan in loader.producers() {
        let collector = Rc::new(RefCell::new(Collector {
            file_num: css::file_map::get_file_num_by_filename(&plan.filename),
            placeholder_owner: Some(owners.len()),
            imported_atoms: entry.borrow().imported_atoms.clone(),
            imported_references: entry.borrow().imported_references.clone(),
            ..Collector::default()
        }));
        register_vanilla_extract_apis(context, &collector)?;
        let mock = context
            .global_object()
            .get(JsString::from(PACKAGE_BINDING), context)
            .map_err(|error| error.to_string())?;
        context
            .register_global_property(
                JsString::from(plan.namespace.as_str()),
                mock,
                Attribute::empty(),
            )
            .map_err(|error| error.to_string())?;
        let index = owners.len();
        context
            .register_global_builtin_callable(
                JsString::from(format!("{}$finish", plan.namespace)),
                1,
                NativeFunction::from_copy_closure(move |_this, args, context| {
                    let mut state = context.remove_data::<State>().ok_or_else(|| {
                        boa_engine::JsNativeError::typ().with_message("missing demand session")
                    })?;
                    let result = state.finish(index, args.get_or_undefined(0), context);
                    context.insert_data(*state);
                    result.map(|()| JsValue::undefined())
                }),
            )
            .map_err(|error| error.to_string())?;
        owners.push(Owner {
            plan: plan.clone(),
            collector,
            finalized: false,
            option: loader.option().clone(),
        });
    }
    for environment in loader.environments() {
        context
            .register_global_builtin_callable(
                JsString::from(format!("{}$read", environment.namespace)),
                1,
                NativeFunction::from_fn_ptr(read),
            )
            .map_err(|error| error.to_string())?;
    }
    let audits = audit::prepare(context, loader.environments())?;
    context.insert_data(State {
        owners,
        audits,
        entry: entry.clone(),
        terminal: Default::default(),
        atoms: loader.imported_atoms.clone(),
        references: loader.imported_references.clone(),
        artifacts: Default::default(),
    });
    register_vanilla_extract_apis(context, entry)
}

fn read(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let mut state = context
        .remove_data::<State>()
        .ok_or_else(|| boa_engine::JsNativeError::typ().with_message("missing demand session"))?;
    let result = state.terminal.resolve(args.get_or_undefined(0), context);
    context.insert_data(*state);
    result
}

pub(super) fn finish(context: &mut Context, imports: &mut StylesheetImports) -> Result<(), String> {
    let state = context
        .remove_data::<State>()
        .ok_or_else(|| "missing final demand session".to_string())?;
    imports.atoms.merge(state.atoms);
    imports.references.merge(state.references);
    imports.artifacts.merge(state.artifacts)?;
    Ok(())
}

pub(super) fn validate(context: &Context) -> Result<(), String> {
    let state = context
        .get_data::<State>()
        .ok_or_else(|| "missing final demand session".to_string())?;
    if let Some(owner) = state.owners.iter().find(|owner| !owner.finalized) {
        return Err(format!(
            "{}:1:1: native producer did not initialize. Fix: retain its original initialization schedule",
            owner.plan.filename
        ));
    }
    if let Some(audit) = state.audits.iter().find(|audit| !audit.checked) {
        return Err(format!(
            "{}:1:1: required selected-data mutation audit did not initialize. Fix: retain its exact module schedule",
            audit.plan.filename
        ));
    }
    Ok(())
}

pub(super) fn resolve(context: &mut Context, value: JsValue) -> JsResult<JsValue> {
    if context.get_data::<State>().is_some() {
        read(&JsValue::undefined(), std::slice::from_ref(&value), context)
    } else {
        Ok(value)
    }
}
