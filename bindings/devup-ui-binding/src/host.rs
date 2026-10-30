use crate::output::NativeOutput;
use crate::value::{Access, AccessCallback, decode, decode_preflight, schema_error};
use devup_ui_binding_core as core;
use extractor::{ExtractOption, ImportAlias, ModuleResolver, ResolvedModule};
use napi::bindgen_prelude::{Array, Either, FnArgs, FromNapiValue, FunctionRef, Unknown};
use napi::{Env, Error, Result};
use napi_derive::napi;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Fresh private resolver return; no original callback result reaches native code.
#[napi(object)]
pub struct Resolved {
    /// Resolved physical filename.
    pub path: String,
    /// Resolved source contents.
    pub code: String,
}
type Resolver = FunctionRef<FnArgs<(String, String)>, Option<Either<String, Resolved>>>;
struct State(Option<Rc<Resolver>>);

/// Initialize environment-local ownership without overwriting existing state.
#[napi]
pub fn init(env: Env) -> Result<()> {
    if env.get_instance_data::<State>()?.is_none() {
        env.set_instance_data(State(None), (), |context| drop(context.value))?;
    }
    Ok(())
}
/// Replace the owned synchronous resolver in the current environment only.
#[napi]
pub fn set_resolver(env: Env, resolver: Option<Resolver>) -> Result<()> {
    state(&env)?.0 = resolver.map(Rc::new);
    Ok(())
}
fn state(env: &Env) -> Result<&'static mut State> {
    env.get_instance_data()?
        .ok_or_else(|| Error::from_reason("native facade is not initialized"))
}
/// Keep production and full-test reset effects distinct, clearing the resolver in both.
#[napi]
pub fn reset(env: Env, full: bool) -> Result<()> {
    let state = state(&env)?;
    if full {
        core::reset_state_internal(|| state.0 = None);
    } else {
        core::reset_build_state_internal();
        state.0 = None;
    }
    Ok(())
}
fn with_resolver<T>(
    env: Env,
    diagnostics: core::ResolverState,
    action: impl FnOnce(
        Option<&ModuleResolver>,
        &dyn Fn() -> std::result::Result<(), String>,
    ) -> Result<T>,
) -> Result<T> {
    let current = state(&env)?.0.clone();
    let diagnostics = Rc::new(RefCell::new(diagnostics));
    let failure = Rc::new(RefCell::new(None));
    let resolver = current.map(|current| {
        let failure = Rc::clone(&failure);
        let diagnostics = Rc::clone(&diagnostics);
        move |specifier: &str, importer: &str| {
            let request = diagnostics.borrow().request(specifier, importer)?;
            match current.borrow_back(&env).and_then(|callback| {
                callback.call((specifier.to_string(), importer.to_string()).into())
            }) {
                Ok(Some(Either::B(module))) => {
                    let module = ResolvedModule {
                        path: module.path,
                        code: module.code,
                    };
                    diagnostics.borrow_mut().cache(&module, &request);
                    Some(module)
                }
                Ok(Some(Either::A(cause))) => {
                    diagnostics.borrow_mut().record(&request, &cause);
                    None
                }
                Ok(None) => None,
                Err(error) => {
                    *failure.borrow_mut() = Some(error);
                    None
                }
            }
        }
    });
    let check = || {
        if failure.borrow().is_some() {
            return Err("private module resolver transport failed".to_string());
        }
        diagnostics.borrow().check()
    };
    let result = action(
        resolver
            .as_ref()
            .map(|resolver| resolver as &ModuleResolver),
        &check,
    );
    match failure.take() {
        Some(error) => Err(error),
        None => result,
    }
}
fn arg<T: FromNapiValue>(request: &Array<'_>, index: u32) -> Result<T> {
    request
        .get(index)?
        .ok_or_else(|| Error::from_reason("missing private request field"))
}
fn aliases(values: HashMap<String, Option<String>>) -> HashMap<String, ImportAlias> {
    values
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                match value {
                    Some(name) => ImportAlias::DefaultToNamed(name),
                    None => ImportAlias::NamedToNamed,
                },
            )
        })
        .collect()
}
/// Check extraction eligibility with forgiving aliases and this environment's resolver.
///
/// # Errors
/// Returns an error if the private request or environment cannot be read.
#[napi]
pub fn has_devup_ui<'env>(
    env: Env,
    request: Array<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<bool> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    let filename: String = arg(&request, 0)?;
    let code: String = arg(&request, 1)?;
    let package: String = arg(&request, 2)?;
    let aliases = aliases(decode_preflight(arg(&request, 3)?, &access)?);
    with_resolver(
        env,
        core::ResolverState::new(&filename, &code),
        |resolver, check| {
            let result = core::has_devup_ui_with(&filename, &code, &package, &aliases, resolver);
            check().map_err(Error::from_reason)?;
            Ok(result)
        },
    )
}
/// Private positional request retains both public eight-argument extraction signatures.
#[napi]
pub fn extract<'env>(
    env: Env,
    request: Array<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<NativeOutput> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    let filename: String = arg(&request, 0)?;
    let code: String = arg(&request, 1)?;
    let option = ExtractOption {
        package: arg(&request, 2)?,
        css_dir: arg(&request, 3)?,
        single_css: arg(&request, 4)?,
        import_main_css: arg(&request, 5)?,
        import_aliases: aliases(decode(arg(&request, 7)?, &access, 7)?),
    };
    let import_main_css_in_css = arg(&request, 6)?;
    let source_map = if arg(&request, 8)? {
        core::SourceMapMode::Generate
    } else {
        core::SourceMapMode::Skip
    };
    with_resolver(
        env,
        core::ResolverState::new(&filename, &code),
        |resolver, check| {
            core::code_extract(
                (&filename, &code),
                option,
                core::ExtractionMode {
                    source_map,
                    resolver,
                    resolver_check: Some(check),
                    import_main_css_in_css,
                },
            )
            .map(|output| NativeOutput(Some(output)))
            .map_err(Error::from_reason)
        },
    )
}
/// Prepare the entire reservation packet before any core allocation or scratch state.
#[napi]
pub fn reserve<'env>(
    env: Env,
    request: Array<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    let value = arg(&request, 0)?;
    let rows: Vec<Vec<String>> = decode(value, &access, 0)?;
    let inputs = rows
        .into_iter()
        .map(|row| {
            let [filename, code, css_dir]: [String; 3] = row
                .try_into()
                .map_err(|_: Vec<String>| schema_error(value, &access, 0))?;
            Ok(core::ReservationInput {
                filename,
                code,
                css_dir,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let option = ExtractOption {
        package: arg(&request, 1)?,
        single_css: arg(&request, 2)?,
        import_main_css: arg(&request, 3)?,
        import_aliases: aliases(decode(arg(&request, 5)?, &access, 5)?),
        ..ExtractOption::default()
    };
    let mut diagnostics = core::ResolverState::default();
    for input in &inputs {
        diagnostics.add_source(&input.filename, &input.code);
    }
    with_resolver(env, diagnostics, |resolver, _check| {
        core::reserve_class_slots_internal(&inputs, option, resolver);
        Ok(())
    })
}
