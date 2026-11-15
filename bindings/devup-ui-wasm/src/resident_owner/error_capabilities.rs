//! Raw callable capabilities, not a platform implementation. No error policy lives here.
use std::{cell::LazyCell, thread::LocalKey};

pub(super) struct Values<V, O> {
    pub undefined: V,
    pub truth: V,
    pub is_object: fn(&V) -> bool,
    pub is_undefined: fn(&V) -> bool,
    pub object: fn(&V) -> &O,
    pub string: fn(&str) -> V,
}

pub(super) struct Arrays<V, A> {
    pub new: fn() -> A,
    pub from: fn(&V) -> A,
    pub push: fn(&A, &V) -> u32,
    pub get: fn(&A, u32) -> V,
    pub set: fn(&A, u32, V),
    // Four inputs are the dependency's Array.of primitive, not an owned policy API.
    pub of4: fn(&V, &V, &V, &V) -> A,
}

pub(super) struct Properties<V, O, D> {
    pub descriptor: fn(&V) -> D,
    pub configurable: fn(&D, bool),
    pub writable: fn(&D, bool),
    pub object: fn(&D) -> &O,
    pub define: fn(&O, &V, &O) -> O,
}

pub(super) struct Errors<E> {
    pub new: fn(&str) -> E,
    pub name: fn(&E, &str),
}

pub(super) struct Authentication<V, O, M: 'static> {
    // LazyCell has no RefCell borrow guard across a host operation. Keys stay weak
    // in the dependency WeakMap; Rust never owns a strong error-identity map.
    pub registry: &'static LocalKey<LazyCell<M>>,
    pub get: fn(&M, &O) -> V,
    pub set: fn(&M, &O, &V) -> M,
}

pub(super) struct Importer {
    pub retain: fn(),
    pub current: fn() -> Option<String>,
}

pub(crate) struct Transport<V, O, A, D, E, M: 'static> {
    pub(super) values: Values<V, O>,
    pub(super) arrays: Arrays<V, A>,
    pub(super) properties: Properties<V, O, D>,
    pub(super) errors: Errors<E>,
    pub(super) authentication: Authentication<V, O, M>,
    pub(super) importer: Importer,
}
