use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{LazyLock, Mutex},
};

use css::resident_owner::{ExportPlan, OwnerFault, PreservationPlan};

use super::{
    access::{Access, Store},
    bank::Bank,
    checkpoint::HostCheckpoint,
    error_native_tests::{NativeErrors, TRANSPORT, Value, setup},
    interpreter::{Call, Context, Decoder, Engine, Owner},
};

pub(super) type Capture = Rc<dyn Fn() -> Result<Value, Value>>;
pub(super) static STORE: Store = LazyLock::new(|| Mutex::new(Bank::new()));
pub(super) const ACCESS: Access<'static> = Access { store: &STORE };

thread_local! {
    pub(super) static CALLBACKS: RefCell<BTreeMap<u64, Capture>> = const { RefCell::new(BTreeMap::new()) };
    pub(super) static EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    pub(super) static FIELD_FAILURE: RefCell<Option<(String, Value)>> = const { RefCell::new(None) };
}

pub(super) const OWNER: Owner<'static, NativeErrors, Capture, Value> = Owner {
    bank: ACCESS,
    callbacks: &CALLBACKS,
    errors: TRANSPORT,
    call: Call {
        invoke: call,
        receiver: Value::Undefined,
    },
    decoder: Decoder {
        get,
        string: Value::text,
        as_string: Value::as_string,
        type_error,
    },
    engine: Engine { reset, restore },
    context: Context {
        retain_importer: retain,
        attach_cause: attach,
        clear_cause: clear,
    },
};

pub(super) struct Fixture {
    _state: crate::TestStateGuard,
}

impl Fixture {
    pub fn new() -> Result<Self, OwnerFault> {
        let guard = crate::TestStateGuard::new();
        *STORE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Ok(Bank::new()?);
        CALLBACKS.with_borrow_mut(BTreeMap::clear);
        FIELD_FAILURE.with_borrow_mut(|value| *value = None);
        EVENTS.with_borrow_mut(Vec::clear);
        setup(None);
        crate::reset_build_state_internal();
        Ok(Self { _state: guard })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        CALLBACKS.with_borrow_mut(BTreeMap::clear);
        FIELD_FAILURE.with_borrow_mut(|failure| *failure = None);
    }
}

fn call(capture: &Capture, receiver: &Value) -> Result<Value, Value> {
    assert_eq!(*receiver, Value::Undefined);
    assert!(STORE.try_lock().is_ok(), "callback must not hold bank lock");
    CALLBACKS.with_borrow_mut(|_| ());
    capture()
}

fn get(value: &Value, key: &Value) -> Result<Value, Value> {
    let Value::Text(key) = key else {
        panic!("reflection key must be text")
    };
    EVENTS.with_borrow_mut(|events| events.push(key.clone()));
    if let Some((field, failure)) = FIELD_FAILURE.with_borrow(Clone::clone)
        && field == *key
    {
        return Err(failure);
    }
    Ok(value.get(key))
}

fn type_error(message: &str) -> Value {
    let error = Value::error(message);
    error.name("TypeError");
    error
}

fn reset() {
    assert!(STORE.try_lock().is_ok(), "reset must not hold bank lock");
    EVENTS.with_borrow_mut(|events| events.push("reset".into()));
    crate::reset_build_state_internal();
}

fn restore(checkpoint: &HostCheckpoint) {
    assert!(STORE.try_lock().is_ok(), "restore must not hold bank lock");
    EVENTS.with_borrow_mut(|events| events.push("restore".into()));
    checkpoint.restore();
}

fn retain() {
    EVENTS.with_borrow_mut(|events| events.push("retain".into()));
}
fn attach(value: Value) -> Value {
    EVENTS.with_borrow_mut(|events| events.push("attach".into()));
    value
}
fn clear() {
    EVENTS.with_borrow_mut(|events| events.push("clear".into()));
}

pub(super) fn ok<T>(result: Result<T, Value>) -> T {
    result.unwrap_or_else(|error| panic!("unexpected native transport failure: {error:?}"))
}

pub(super) fn fault<T>(result: Result<T, Value>, expected: OwnerFault) -> Value {
    let Err(error) = result else {
        panic!("expected native failure {expected:?}")
    };
    assert_eq!(
        TRANSPORT.details(&error).at(0),
        Value::text(&format!("{expected:?}"))
    );
    error
}

pub(super) fn parts() -> Result<[String; 5], String> {
    Ok([
        crate::export_sheet_internal()?,
        crate::export_class_map_internal()?,
        crate::export_file_map_internal()?,
        crate::export_canonical_map_internal()?,
        crate::reservation::export_reservation_state_internal()?,
    ])
}

pub(super) fn envelope(parts: [String; 5]) -> Value {
    let value = Value::new();
    for (key, text) in ["sheet", "classes", "files", "canonical", "reservation"]
        .into_iter()
        .zip(parts)
    {
        value.set_property(key, Value::text(&text));
    }
    value
}

pub(super) fn snapshot() -> Result<Value, Value> {
    parts()
        .map(envelope)
        .map_err(|message| Value::error(&message))
}
pub(super) fn create() -> u64 {
    ok(OWNER.create(Rc::new(snapshot)))
}
pub(super) fn run(id: u64) {
    ok(OWNER.begin(id));
    ok(OWNER.configured());
    ok(OWNER.finish());
}

pub(super) fn preservation(id: u64) -> Result<PreservationPlan, OwnerFault> {
    run(id);
    ACCESS.with(|bank| {
        match bank
            .core
            .begin_export(css::resident_owner::ExportKind::Mutate)?
        {
            ExportPlan::Preserve(plan) => Ok(plan),
            ExportPlan::Admitted(_) => panic!("resident must require preservation"),
        }
    })
}
