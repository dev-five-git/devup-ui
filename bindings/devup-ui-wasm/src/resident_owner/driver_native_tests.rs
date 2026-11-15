use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

pub(super) trait Required {
    type Output;
    fn required(self) -> Self::Output;
}
impl<T, E: fmt::Debug> Required for Result<T, E> {
    type Output = T;
    fn required(self) -> T {
        self.unwrap_or_else(|error| panic!("unexpected test failure: {error:?}"))
    }
}
impl<T> Required for Option<T> {
    type Output = T;
    fn required(self) -> T {
        self.unwrap_or_else(|| panic!("expected test value"))
    }
}

use super::{
    BANK,
    access::Access,
    bank::Bank,
    driver_value_tests::{ERRORS, NativeErrors, attach, error},
};
use crate::{
    driver::{Arguments, Conversion, Driver, Errors, Source},
    resolver::native_tests::{self, Callback, Value},
};

thread_local! {
    pub(super) static INSTALLED: RefCell<Option<Callback>> = const { RefCell::new(None) };
    pub(super) static CONVERSIONS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

pub(super) const DRIVER: Driver<Value, Callback, Value, DecodeError, Value, NativeErrors> =
    Driver {
        resolver: native_tests::RESOLVER,
        installed: &INSTALLED,
        conversion: Conversion {
            aliases,
            strings,
            rows,
        },
        errors: Errors {
            new: error,
            transport: ERRORS,
            structural: super::Transport::error,
            attach,
            fault: super::check_fault,
        },
        export_active: super::export_active,
    };

#[derive(Debug)]
pub(super) struct DecodeError(serde_json::Error);
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
fn aliases(value: Value) -> Result<HashMap<String, Option<String>>, DecodeError> {
    CONVERSIONS.with_borrow_mut(|events| events.push("aliases"));
    serde_json::from_value(value.json_value().unwrap_or(serde_json::Value::Null))
        .map_err(DecodeError)
}
fn strings(value: Value) -> Result<Vec<String>, DecodeError> {
    CONVERSIONS.with_borrow_mut(|events| events.push("strings"));
    serde_json::from_value(value.json_value().unwrap_or(serde_json::Value::Null))
        .map_err(DecodeError)
}
fn rows(value: Value) -> Result<Vec<Vec<String>>, DecodeError> {
    CONVERSIONS.with_borrow_mut(|events| events.push("rows"));
    serde_json::from_value(value.json_value().unwrap_or(serde_json::Value::Null))
        .map_err(DecodeError)
}

pub(super) struct Fixture {
    previous: Option<Result<Bank, css::resident_owner::OwnerFault>>,
    _engine: crate::TestStateGuard,
    _resolver: native_tests::Fixture,
}
impl Fixture {
    pub fn new() -> Self {
        let engine = crate::TestStateGuard::new();
        let previous = std::mem::replace(&mut *BANK.lock().required(), Bank::new());
        INSTALLED.with_borrow_mut(|value| *value = None);
        CONVERSIONS.with_borrow_mut(Vec::clear);
        crate::reset_build_state_internal();
        super::driver_value_tests::CREATED.set(0);
        Self {
            previous: Some(previous),
            _engine: engine,
            _resolver: native_tests::Fixture::new(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        *BANK.lock().required() = self.previous.take().required();
        INSTALLED.with_borrow_mut(|value| *value = None);
    }
}

pub(super) fn install(callback: impl Fn(&Value, &Value) -> Result<Value, Value> + 'static) {
    DRIVER.install(Some(Rc::new(move |request, importer| {
        assert!(BANK.try_lock().is_ok());
        INSTALLED.with_borrow_mut(|_| ());
        native_tests::unborrowed();
        callback(request, importer)
    })));
}
pub(super) fn admit() {
    Access { store: &BANK }
        .with(|bank| {
            let (plan, id) = bank.enter_export(0)?;
            let css::resident_owner::ExportPlan::Admitted(ticket) = plan else {
                panic!("unowned admission")
            };
            bank.export = Some((id, ticket));
            Ok(())
        })
        .required();
}
pub(super) fn reenter() -> css::resident_owner::OwnerFault {
    Access { store: &BANK }
        .with(|bank| bank.enter_export(0))
        .err()
        .required()
}
pub(super) fn source<'a>(filename: &'a str, code: &'a str) -> Source<'a> {
    Source {
        filename,
        code,
        option: extractor::ExtractOption {
            single_css: true,
            ..extractor::ExtractOption::default()
        },
        import_main_css_in_css: false,
        map_mode: crate::SourceMapMode::Skip,
    }
}
pub(super) fn args() -> Arguments<Value> {
    Arguments {
        aliases: Value::object(&[]),
        source_type: Value::Undefined,
        opaque_defaults: Value::Undefined,
    }
}
pub(super) fn list(values: Vec<Value>) -> Value {
    Value::Array(Rc::new(RefCell::new(values)))
}
pub(super) fn row(filename: &str, code: &str) -> Value {
    list(vec![
        Value::text(filename),
        Value::text(code),
        Value::text("./css"),
    ])
}
pub(super) fn module(path: &str, code: &str) -> Value {
    Value::object(&[("path", Value::text(path)), ("code", Value::text(code))])
}
pub(super) fn message(error: &Value) -> String {
    error.get("message").required().as_string().required()
}
