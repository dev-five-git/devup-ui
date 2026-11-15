use std::{
    cell::{Cell, LazyCell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

use super::driver_native_tests::Required;
use super::error_capabilities::{
    Arrays, Authentication, Errors, Importer, Properties, Transport, Values,
};
use crate::resolver::{native_tests::STATE, value_tests::Value};

type Key = RefCell<BTreeMap<String, Value>>;
type Entry = (Weak<Key>, Value);
#[derive(Clone, Default)]
pub(super) struct Registry(Rc<RefCell<Vec<Entry>>>);

impl Registry {
    fn get(&self, key: &Value) -> Value {
        let Value::Object(key) = key else {
            panic!("WeakMap key")
        };
        self.0
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate.ptr_eq(&Rc::downgrade(key)))
            .map_or(Value::Undefined, |(_, value)| value.clone())
    }
    fn set(&self, key: &Value, value: &Value) -> Self {
        let Value::Object(key) = key else {
            panic!("WeakMap key")
        };
        self.0.borrow_mut().retain(|(candidate, _)| {
            candidate.strong_count() > 0 && !candidate.ptr_eq(&Rc::downgrade(key))
        });
        self.0
            .borrow_mut()
            .push((Rc::downgrade(key), value.clone()));
        self.clone()
    }
}

thread_local! {
    static REGISTRY: LazyCell<Registry> = const { LazyCell::new(Registry::default) };
    pub(super) static CREATED: Cell<usize> = const { Cell::new(0) };
}

pub(super) type NativeErrors = Transport<Value, Value, Value, Value, Value, Registry>;
pub(super) const ERRORS: NativeErrors = Transport {
    values: Values {
        undefined: Value::Undefined,
        truth: Value::Bool(true),
        is_object: Value::is_object,
        is_undefined: Value::is_undefined,
        object: view,
        string: Value::text,
    },
    arrays: Arrays {
        new: array,
        from: copy,
        push,
        get,
        set,
        of4,
    },
    properties: Properties {
        descriptor,
        configurable,
        writable,
        object: view,
        define,
    },
    errors: Errors { new: error, name },
    authentication: Authentication {
        registry: &REGISTRY,
        get: Registry::get,
        set: Registry::set,
    },
    importer: Importer { retain, current },
};

fn view(value: &Value) -> &Value {
    value
}
fn array() -> Value {
    Value::Array(Rc::new(RefCell::new(Vec::new())))
}
fn copy(value: &Value) -> Value {
    let Value::Array(values) = value else {
        panic!("Array.from")
    };
    Value::Array(Rc::new(RefCell::new(values.borrow().clone())))
}
fn push(array: &Value, value: &Value) -> u32 {
    let Value::Array(values) = array else {
        panic!("Array.push")
    };
    let mut values = values.borrow_mut();
    values.push(value.clone());
    u32::try_from(values.len()).required()
}
fn get(array: &Value, index: u32) -> Value {
    let Value::Array(values) = array else {
        panic!("Array.get")
    };
    values
        .borrow()
        .get(usize::try_from(index).required())
        .cloned()
        .unwrap_or(Value::Undefined)
}
fn set(array: &Value, index: u32, value: Value) {
    let Value::Array(values) = array else {
        panic!("Array.set")
    };
    let index = usize::try_from(index).required();
    let mut values = values.borrow_mut();
    let length = values.len().max(index + 1);
    values.resize(length, Value::Undefined);
    values[index] = value;
}
fn of4(a: &Value, b: &Value, c: &Value, d: &Value) -> Value {
    Value::Array(Rc::new(RefCell::new(vec![
        a.clone(),
        b.clone(),
        c.clone(),
        d.clone(),
    ])))
}
fn descriptor(value: &Value) -> Value {
    Value::object(&[("value", value.clone())])
}
fn configurable(descriptor: &Value, value: bool) {
    descriptor.set("configurable", Value::Bool(value));
}
fn writable(descriptor: &Value, value: bool) {
    descriptor.set("writable", Value::Bool(value));
}
fn define(object: &Value, key: &Value, descriptor: &Value) -> Value {
    object.set(
        &key.as_string().required(),
        descriptor.get("value").required(),
    );
    object.clone()
}
pub(super) fn error(message: &str) -> Value {
    CREATED.set(CREATED.get() + 1);
    Value::object(&[("message", Value::text(message))])
}
fn name(error: &Value, name: &str) {
    error.set("name", Value::text(name));
}
fn retain() {
    STATE.with(crate::resolver::State::retain_importer);
}
fn current() -> Option<String> {
    STATE.with(crate::resolver::State::importer)
}
pub(super) fn attach(error: Value) -> Value {
    STATE.with(|state| state.attach_cause(error, &ERRORS, Transport::attach_cause))
}

#[test]
fn primitives_copy_arrays_and_registry_does_not_retain_keys() {
    // Given
    let values = of4(
        &Value::Null,
        &Value::Undefined,
        &Value::Bool(true),
        &Value::text("a"),
    );
    let registry = Registry::default();
    let error = error("native");
    let Value::Object(key) = &error else {
        unreachable!()
    };
    let weak = Rc::downgrade(key);
    // When
    let copied = copy(&values);
    set(&values, 3, Value::text("b"));
    registry.set(&error, &copied);
    // Then
    assert_eq!(registry.get(&error), copied);
    assert_eq!(get(&copied, 3), Value::text("a"));
    drop(error);
    assert!(weak.upgrade().is_none());
}
