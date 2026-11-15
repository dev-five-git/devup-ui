//! Native export policy. Platform capabilities perform representation operations only.
mod error;
mod extract;
mod resolve;
mod wiring;

use std::{cell::RefCell, collections::HashMap, thread::LocalKey};

use extractor::{ExtractOption, ImportAlias};

pub(crate) use error::{Errors, message_error};
pub(crate) use extract::{Arguments, Source};
pub(crate) use resolve::fallible_resolver;
pub(crate) use wiring::DRIVER;

type AliasInput = HashMap<String, Option<String>>;

pub(crate) struct Conversion<V, C> {
    pub aliases: fn(V) -> Result<AliasInput, C>,
    pub strings: fn(V) -> Result<Vec<String>, C>,
    pub rows: fn(V) -> Result<Vec<Vec<String>>, C>,
}

pub(crate) struct ReservationOptions<'a, V> {
    pub package: &'a str,
    pub single_css: bool,
    pub import_main_css: bool,
    pub aliases: V,
}

pub(crate) struct Driver<V: 'static, F: 'static, S, C, E, T> {
    pub resolver: crate::resolver::Resolver<V, F, S>,
    pub installed: &'static LocalKey<RefCell<Option<F>>>,
    pub conversion: Conversion<V, C>,
    pub errors: Errors<V, E, T>,
    pub export_active: fn() -> bool,
}

impl<V: Clone, F: Clone, S, C: std::fmt::Display, E: Into<V>, T> Driver<V, F, S, C, E, T> {
    pub(crate) fn install(&self, resolver: Option<F>) {
        self.installed
            .with_borrow_mut(|current| *current = resolver);
    }

    pub(crate) fn aliases(&self, value: V) -> Result<HashMap<String, ImportAlias>, V> {
        let aliases =
            (self.conversion.aliases)(value).map_err(|error| self.errors.message(error))?;
        Ok(aliases
            .into_iter()
            .map(|(key, value)| {
                let alias = match value {
                    Some(name) => ImportAlias::DefaultToNamed(name),
                    None => ImportAlias::NamedToNamed,
                };
                (key, alias)
            })
            .collect())
    }

    pub(crate) fn reserve(&self, rows: V, options: ReservationOptions<'_, V>) -> Result<(), V> {
        let _scope = self.scope();
        let rows = (self.conversion.rows)(rows).map_err(|error| self.errors.message(error))?;
        let inputs = crate::reservation::rows::parse_rows(rows)
            .map_err(|error| self.errors.message(error))?;
        let option = ExtractOption {
            package: options.package.into(),
            single_css: options.single_css,
            import_main_css: options.import_main_css,
            import_aliases: self.aliases(options.aliases)?,
            ..ExtractOption::default()
        };
        let installed = self.installed.with_borrow(Clone::clone);
        match installed {
            Some(resolver) => {
                crate::reservation::reserve_class_slots_with(&inputs, option, |input, option| {
                    let _row_scope = self.scope();
                    crate::reservation::extract_scratch_row(
                        input,
                        option,
                        &|specifier, importer| self.resolver.call(&resolver, specifier, importer),
                    )
                });
            }
            None => crate::reservation::reserve_class_slots_internal(&inputs, option, None),
        }
        self.errors.check()
    }

    fn scope(&self) -> crate::resolver::Scope<V> {
        crate::resolver::Scope::new(self.resolver.state, self.export_active)
    }
}
