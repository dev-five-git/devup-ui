use std::thread::LocalKey;

use super::state::State;

pub(crate) struct Values<V> {
    pub is_null: fn(&V) -> bool,
    pub is_undefined: fn(&V) -> bool,
    pub is_object: fn(&V) -> bool,
    pub as_bool: fn(&V) -> Option<bool>,
    pub as_string: fn(&V) -> Option<String>,
    pub string: fn(&str) -> V,
}

pub(crate) struct Call<F, V> {
    // The four raw operands are the dependency's Function::call2 signature.
    pub invoke: fn(&F, &V, &V, &V) -> Result<V, V>,
    pub receiver: V,
}

pub(crate) struct Json<V, S> {
    pub stringify: fn(&V) -> Result<S, V>,
    pub value: fn(&S) -> &V,
}

pub(crate) struct Resolver<V: 'static, F, S> {
    pub state: &'static LocalKey<State<V>>,
    pub values: Values<V>,
    pub call: Call<F, V>,
    pub get: fn(&V, &V) -> Result<V, V>,
    pub json: Json<V, S>,
}
