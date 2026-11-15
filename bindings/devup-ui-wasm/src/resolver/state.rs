use std::{cell::RefCell, thread::LocalKey};

pub(crate) struct State<V> {
    pub(super) cause: RefCell<Option<V>>,
    pub(super) current: RefCell<Option<String>>,
    pub(super) origin: RefCell<Option<String>>,
}

impl<V> State<V> {
    pub const fn new() -> Self {
        Self {
            cause: RefCell::new(None),
            current: RefCell::new(None),
            origin: RefCell::new(None),
        }
    }

    pub fn clear_cause(&self) {
        *self.cause.borrow_mut() = None;
        *self.origin.borrow_mut() = None;
    }

    pub fn take_cause(&self) -> Option<V> {
        self.cause.borrow_mut().take()
    }

    pub fn attach_cause<E>(&self, error: V, transport: &E, attach: fn(&E, V, V) -> V) -> V {
        match self.take_cause() {
            Some(cause) => attach(transport, error, cause),
            None => error,
        }
    }

    pub fn importer(&self) -> Option<String> {
        self.origin
            .borrow()
            .clone()
            .or_else(|| self.current.borrow().clone())
    }

    pub fn retain_importer(&self) {
        let current = self.current.borrow().clone();
        let mut origin = self.origin.borrow_mut();
        if origin.is_none() {
            *origin = current;
        }
    }

    pub fn enter(&self, importer: &str) -> ImporterScope<'_, V> {
        let previous = self.current.borrow_mut().replace(importer.to_owned());
        ImporterScope {
            state: self,
            previous,
        }
    }
}

impl<V: Clone> State<V> {
    pub fn remember(&self, value: &V) {
        self.retain_importer();
        let mut cause = self.cause.borrow_mut();
        if cause.is_none() {
            *cause = Some(value.clone());
        }
    }
}

pub(crate) struct ImporterScope<'a, V> {
    state: &'a State<V>,
    previous: Option<String>,
}

impl<V> Drop for ImporterScope<'_, V> {
    fn drop(&mut self) {
        *self.state.current.borrow_mut() = self.previous.take();
    }
}

pub(crate) struct FaultScope<V: 'static> {
    state: &'static LocalKey<State<V>>,
    export_active: fn() -> bool,
}

impl<V> FaultScope<V> {
    pub fn new(state: &'static LocalKey<State<V>>, export_active: fn() -> bool) -> Self {
        state.with(State::clear_cause);
        Self {
            state,
            export_active,
        }
    }
}

impl<V> Drop for FaultScope<V> {
    fn drop(&mut self) {
        if !(self.export_active)() {
            self.state.with(State::clear_cause);
        }
    }
}
