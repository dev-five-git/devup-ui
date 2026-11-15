use std::{cell::RefCell, collections::BTreeMap, thread::LocalKey};

use css::resident_owner::OwnerFault;

use super::{access::Access, checkpoint::HostCheckpoint, decision::Faults};

pub(super) trait ErrorTransport {
    type Value: Clone;
    fn error(&self, fault: OwnerFault) -> Self::Value;
    fn message(&self, message: &str) -> Self::Value;
    fn with_cause(&self, fault: OwnerFault, primary: Self::Value) -> Self::Value;
    fn cleanup(&self, primary: Self::Value, cleanup: Self::Value) -> Self::Value;
    fn faults(&self, faults: Faults) -> Self::Value;
    fn complete<T>(
        &self,
        result: Result<T, Self::Value>,
        unwind: Result<(), Self::Value>,
    ) -> Result<T, Self::Value>;
}

pub(super) struct Decoder<V, T> {
    pub get: fn(&V, &V) -> Result<V, V>,
    pub string: fn(&str) -> V,
    pub as_string: fn(&V) -> Option<String>,
    pub type_error: fn(&str) -> T,
}

pub(super) struct Call<F, V> {
    pub invoke: fn(&F, &V) -> Result<V, V>,
    pub receiver: V,
}

pub(super) struct Engine {
    pub reset: fn(),
    pub restore: fn(&HostCheckpoint),
}

pub(super) struct Context<V> {
    pub retain_importer: fn(),
    pub attach_cause: fn(V) -> V,
    pub clear_cause: fn(),
}

pub(super) struct Owner<'a, E: ErrorTransport, F: 'static, T> {
    pub bank: Access<'a>,
    pub callbacks: &'static LocalKey<RefCell<BTreeMap<u64, F>>>,
    pub errors: E,
    pub call: Call<F, E::Value>,
    pub decoder: Decoder<E::Value, T>,
    pub engine: Engine,
    pub context: Context<E::Value>,
}
