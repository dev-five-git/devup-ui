use boa_engine::{Context, JsArgs, JsResult, JsString, JsValue, NativeFunction};

use super::{Binding, State};
use crate::module_loader::demand::Producer;
use crate::vanilla_extract::capture::{FinalizeError, check_mutations};

pub(super) struct Audit {
    pub plan: Producer,
    pub checked: bool,
}

pub(super) fn prepare(
    context: &mut Context,
    environments: &[Producer],
) -> Result<Vec<Audit>, String> {
    let mut audits = Vec::new();
    for plan in environments.iter().filter(|plan| !plan.checks.is_empty()) {
        let index = audits.len();
        context
            .register_global_builtin_callable(
                JsString::from(format!("{}$audit", plan.namespace)),
                1,
                NativeFunction::from_copy_closure(move |_this, args, context| {
                    let mut state = context.remove_data::<State>().ok_or_else(|| {
                        boa_engine::JsNativeError::typ()
                            .with_message("missing demand audit session")
                    })?;
                    let result = state.audit(index, args.get_or_undefined(0), context);
                    context.insert_data(*state);
                    result.map(|()| JsValue::undefined())
                }),
            )
            .map_err(|error| error.to_string())?;
        audits.push(Audit {
            plan: plan.clone(),
            checked: false,
        });
    }
    Ok(audits)
}

impl State {
    fn audit(&mut self, index: usize, array: &JsValue, context: &mut Context) -> JsResult<()> {
        let audit = self.audits.get_mut(index).ok_or_else(|| {
            boa_engine::JsNativeError::range().with_message("unknown demand audit")
        })?;
        let values = crate::vanilla_extract::array_items(array, context)?.ok_or_else(|| {
            boa_engine::JsNativeError::typ().with_message("demand audit values must be an array")
        })?;
        let mut names: Vec<_> = audit
            .plan
            .checks
            .iter()
            .map(|check| check.read.clone())
            .collect();
        names.sort();
        names.dedup();
        if values.len() != names.len() {
            return Err(boa_engine::JsNativeError::typ().with_message(format!("{}:1:1: required mutation audit has missing binding values. Fix: report this extraction error", audit.plan.filename)).into());
        }
        let bindings: Vec<_> = names
            .into_iter()
            .map(|name| Binding {
                name: name.clone(),
                exported: false,
                read: name,
                alias: None,
                init: None,
            })
            .collect();
        let values: Vec<_> = bindings.iter().zip(values).collect();
        check_mutations(&audit.plan.checks, &values, context).map_err(|error| match error {
            FinalizeError::Js(error) => error,
            FinalizeError::Located(message) => boa_engine::JsNativeError::typ()
                .with_message(message)
                .into(),
        })?;
        audit.checked = true;
        Ok(())
    }
}
