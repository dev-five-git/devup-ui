use std::cell::RefCell;

use extractor::{ModuleResolution, ModuleResolver};

/// Adapt a fallible capability without reentering it after the first fault.
pub(crate) fn fallible_resolver<'a>(
    fault: &'a RefCell<Option<String>>,
    resolve: impl Fn(&str, &str) -> Result<Option<ModuleResolution>, String> + 'a,
) -> Box<ModuleResolver<'a>> {
    Box::new(move |specifier, importer| {
        if fault.borrow().is_some() {
            return None;
        }
        match resolve(specifier, importer) {
            Ok(module) => module,
            Err(cause) => {
                *fault.borrow_mut() = Some(format!(
                    "{importer}:1:1: module resolver cannot use `{specifier}` at build time: {cause}"
                ));
                None
            }
        }
    })
}
