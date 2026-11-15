use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(crate) use super::value_tests::Value;
use super::{
    capabilities::{Call, Json, Resolver, Values},
    state::State,
};

pub(crate) type Callback = Rc<dyn Fn(&Value, &Value) -> Result<Value, Value>>;
type Getter = Rc<dyn Fn(&Value, &str) -> Result<Value, Value>>;
type Stringifier = Rc<dyn Fn(&Value) -> Result<Value, Value>>;

thread_local! {
    pub(crate) static STATE: State<Value> = const { State::new() };
    pub(super) static READS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    pub(super) static GETTER: RefCell<Option<Getter>> = const { RefCell::new(None) };
    pub(super) static JSON: RefCell<Option<Stringifier>> = const { RefCell::new(None) };
    pub(super) static ACTIVE: Cell<bool> = const { Cell::new(false) };
    pub(super) static JSON_CALLS: Cell<usize> = const { Cell::new(0) };
}

pub(crate) const RESOLVER: Resolver<Value, Callback, Value> = Resolver {
    state: &STATE,
    values: Values {
        is_null: Value::is_null,
        is_undefined: Value::is_undefined,
        is_object: Value::is_object,
        as_bool: Value::as_bool,
        as_string: Value::as_string,
        string: Value::text,
    },
    call: Call {
        invoke,
        receiver: Value::Null,
    },
    get,
    json: Json {
        stringify,
        value: view,
    },
};

pub(crate) struct Fixture;

impl Fixture {
    pub fn new() -> Self {
        STATE.with(|state| {
            state.clear_cause();
            *state.current.borrow_mut() = None;
        });
        READS.with_borrow_mut(Vec::clear);
        GETTER.with_borrow_mut(|getter| *getter = None);
        JSON.with_borrow_mut(|stringifier| *stringifier = None);
        ACTIVE.set(false);
        JSON_CALLS.set(0);
        Self
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        GETTER.with_borrow_mut(|getter| *getter = None);
        JSON.with_borrow_mut(|stringifier| *stringifier = None);
        STATE.with(|state| {
            state.clear_cause();
            *state.current.borrow_mut() = None;
        });
    }
}

pub(crate) fn unborrowed() {
    STATE.with(|state| {
        assert!(state.cause.try_borrow_mut().is_ok());
        assert!(state.current.try_borrow_mut().is_ok());
        assert!(state.origin.try_borrow_mut().is_ok());
    });
}

pub(crate) fn active() -> bool {
    unborrowed();
    ACTIVE.get()
}
fn view(value: &Value) -> &Value {
    value
}

fn invoke(
    callback: &Callback,
    receiver: &Value,
    request: &Value,
    importer: &Value,
) -> Result<Value, Value> {
    unborrowed();
    assert_eq!(*receiver, Value::Null);
    callback(request, importer)
}

fn get(value: &Value, key: &Value) -> Result<Value, Value> {
    unborrowed();
    let Value::Text(key) = key else {
        panic!("raw reflection key must be text")
    };
    READS.with_borrow_mut(|reads| reads.push(key.clone()));
    let getter = GETTER.with_borrow(Clone::clone);
    match getter {
        Some(getter) => getter(value, key),
        None => value.get(key),
    }
}

fn stringify(value: &Value) -> Result<Value, Value> {
    unborrowed();
    JSON_CALLS.set(JSON_CALLS.get() + 1);
    let stringify = JSON.with_borrow(Clone::clone);
    match stringify {
        Some(stringify) => stringify(value),
        None => value.stringify(),
    }
}

pub(super) fn callback(result: Result<Value, Value>) -> Callback {
    Rc::new(move |_, _| result.clone())
}

pub(super) fn module() -> Value {
    Value::object(&[
        ("path", Value::text("/src/token.ts")),
        ("code", Value::text("export const C = 'red';")),
    ])
}

pub(crate) fn cause() -> Option<Value> {
    STATE.with(|state| state.cause.borrow().clone())
}
pub(super) fn importer() -> Option<String> {
    STATE.with(State::importer)
}
