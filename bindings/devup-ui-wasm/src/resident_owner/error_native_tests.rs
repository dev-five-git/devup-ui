use std::{
    cell::{Cell, LazyCell, RefCell},
    rc::{Rc, Weak},
};

use super::error_capabilities::{
    Arrays, Authentication, Errors, Importer, Properties, Transport, Values,
};
pub(super) use super::native_value_tests::{CREATED, Object, Value};

type Entry = (Weak<Object>, Value);

#[derive(Clone, Default)]
pub(super) struct WeakMap(Rc<RefCell<Vec<Entry>>>);

impl WeakMap {
    fn get(&self, key: &Rc<Object>) -> Value {
        self.0
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate.ptr_eq(&Rc::downgrade(key)))
            .map_or(Value::Undefined, |(_, value)| value.clone())
    }

    fn set(&self, key: &Rc<Object>, value: &Value) -> Self {
        let mut entries = self.0.borrow_mut();
        entries.retain(|(candidate, _)| {
            candidate.strong_count() > 0 && !candidate.ptr_eq(&Rc::downgrade(key))
        });
        entries.push((Rc::downgrade(key), value.clone()));
        self.clone()
    }
}

thread_local! {
    static FAULTS: LazyCell<WeakMap> = const { LazyCell::new(WeakMap::default) };
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
    static RETAINED: Cell<bool> = const { Cell::new(false) };
}

fn retain() {
    RETAINED.set(true);
}
fn importer() -> Option<String> {
    assert!(RETAINED.get());
    CURRENT.with_borrow(Clone::clone)
}

pub(super) type NativeErrors = Transport<Value, Rc<Object>, Value, Value, Value, WeakMap>;

pub(super) const TRANSPORT: NativeErrors = Transport {
    values: Values {
        undefined: Value::Undefined,
        truth: Value::Bool(true),
        is_object: Value::is_object,
        is_undefined: Value::is_undefined,
        object: Value::object,
        string: Value::text,
    },
    arrays: Arrays {
        new: Value::new,
        from: Value::array_from,
        push: Value::push,
        get: Value::at,
        set: Value::set,
        of4: Value::of4,
    },
    properties: Properties {
        descriptor: Value::descriptor,
        configurable: Value::configurable,
        writable: Value::writable,
        object: Value::object,
        define: Value::define,
    },
    errors: Errors {
        new: Value::error,
        name: Value::name,
    },
    authentication: Authentication {
        registry: &FAULTS,
        get: WeakMap::get,
        set: WeakMap::set,
    },
    importer: Importer {
        retain,
        current: importer,
    },
};

pub(super) fn setup(importer: Option<&str>) {
    CURRENT.with_borrow_mut(|current| *current = importer.map(str::to_owned));
    RETAINED.set(false);
    CREATED.set(0);
    FAULTS.with(|map| map.0.borrow_mut().clear());
}

#[test]
fn primitives_copy_arrays_without_retaining_weak_keys() {
    // Given an identity-bearing array and a weak registry entry.
    let map = WeakMap::default();
    let key = Value::new();
    let weak = Rc::downgrade(key.object());
    key.set(0, Value::text("before"));
    let copy = Value::array_from(&key);
    map.set(key.object(), &copy);
    // When the original array changes and its last owner is released.
    key.set(0, Value::text("after"));
    assert_eq!(map.get(key.object()).at(0), Value::text("before"));
    drop(key);
    // Then the registry does not keep its key alive.
    assert!(weak.upgrade().is_none());
}
