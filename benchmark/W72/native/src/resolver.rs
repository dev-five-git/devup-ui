use std::cell::RefCell;
use std::rc::Rc;

use extractor::ResolvedModule;
use napi::Env;
use napi::bindgen_prelude::{FnArgs, FunctionRef};
use napi_derive::napi;

/// The JS entrypoint validates fields and suppresses callback/property exceptions.
#[napi(object)]
pub struct ResolverModule {
    /// The resolved module's path.
    pub path: String,
    /// The resolved module's source code.
    pub code: String,
}

type ResolverRef = FunctionRef<FnArgs<(String, String)>, Option<ResolverModule>>;

thread_local! {
    static MODULE_RESOLVER: RefCell<Option<Rc<ResolverRef>>> = const { RefCell::new(None) };
}

/// Install or clear the synchronous JavaScript module resolver.
#[napi(js_name = "setModuleResolver")]
pub fn set_module_resolver(resolver: Option<ResolverRef>) {
    MODULE_RESOLVER.with_borrow_mut(|current| *current = resolver.map(Rc::new));
}

pub(crate) fn clear() {
    MODULE_RESOLVER.with_borrow_mut(|current| *current = None);
}

pub(crate) fn snapshot(env: Env) -> Option<impl Fn(&str, &str) -> Option<ResolvedModule>> {
    MODULE_RESOLVER.with_borrow(Clone::clone).map(|resolver| {
        move |specifier: &str, importer: &str| {
            // No RefCell borrow or sheet lock spans the synchronous JS callback.
            // Function::call captures/clears a JS exception in its returned Error.
            let module = match resolver.borrow_back(&env).and_then(|callback| {
                callback.call((specifier.to_string(), importer.to_string()).into())
            }) {
                Ok(module) => module?,
                Err(_) => return None, // WASM deliberately treats resolver errors as misses.
            };
            Some(ResolvedModule {
                path: module.path,
                code: module.code,
            })
        }
    })
}
