//! Authenticated fault transport; public Error names carry no authority.
use css::resident_owner::OwnerFault;

use super::{decision::Faults, error_capabilities::Transport};

impl<V: Clone, O, A: Into<V>, D, E: Into<V>, M> Transport<V, O, A, D, E, M> {
    pub(crate) fn error(&self, fault: OwnerFault) -> V {
        let error = (self.errors.new)(&fault.to_string());
        (self.errors.name)(&error, "ResidentOwnerError");
        let details = (self.arrays.new)();
        (self.arrays.push)(&details, &(self.values.string)(&format!("{fault:?}")));
        (self.arrays.push)(&details, &(self.values.string)(&fault.to_string()));
        (self.importer.retain)();
        let importer = (self.importer.current)().map_or_else(
            || self.values.undefined.clone(),
            |text| (self.values.string)(&text),
        );
        (self.arrays.push)(&details, &importer);
        let error = error.into();
        self.authenticate(&error, details);
        error
    }

    fn authenticate(&self, error: &V, details: A) {
        self.authentication.registry.with(|faults| {
            (self.authentication.set)(faults, (self.values.object)(error), &details.into());
        });
    }

    pub(super) fn details(&self, value: &V) -> V {
        if (self.values.is_object)(value) {
            self.authentication
                .registry
                .with(|faults| (self.authentication.get)(faults, (self.values.object)(value)))
        } else {
            self.values.undefined.clone()
        }
    }

    pub(super) fn with_cause(&self, fault: OwnerFault, primary: V) -> V {
        self.attach_cause(self.error(fault), primary)
    }

    pub(crate) fn attach_cause(&self, error: V, cause: V) -> V {
        self.property(&error, "cause", &cause);
        let details = self.details(&error);
        if !(self.values.is_undefined)(&details) {
            let details = (self.arrays.from)(&details);
            (self.arrays.set)(&details, 4, self.values.truth.clone());
            (self.arrays.set)(&details, 5, cause);
            self.authenticate(&error, details);
        }
        error
    }

    fn property(&self, error: &V, key: &str, value: &V) {
        let descriptor = (self.properties.descriptor)(value);
        (self.properties.configurable)(&descriptor, true);
        (self.properties.writable)(&descriptor, true);
        (self.properties.define)(
            (self.values.object)(error),
            &(self.values.string)(key),
            (self.properties.object)(&descriptor),
        );
    }

    pub(super) fn cleanup(&self, primary: V, cleanup: V) -> V {
        let failure = (self.errors.new)("engine operation failed during cleanup").into();
        let mut cleanup = cleanup;
        let details = self.details(&primary);
        if !(self.values.is_undefined)(&details) {
            let details = (self.arrays.from)(&details);
            let previous = (self.arrays.get)(&details, 3);
            if !(self.values.is_undefined)(&previous) {
                cleanup = self.cleanup(previous, cleanup);
            }
            let copied = (self.arrays.of4)(
                &(self.arrays.get)(&details, 0),
                &(self.arrays.get)(&details, 1),
                &(self.arrays.get)(&details, 2),
                &cleanup,
            );
            (self.arrays.set)(&copied, 4, (self.arrays.get)(&details, 4));
            (self.arrays.set)(&copied, 5, (self.arrays.get)(&details, 5));
            self.authenticate(&failure, copied);
        }
        for (key, value) in [("cause", primary), ("cleanupCause", cleanup)] {
            self.property(&failure, key, &value);
        }
        failure
    }

    pub(super) fn complete<T>(&self, result: Result<T, V>, unwind: Result<(), V>) -> Result<T, V> {
        match (result, unwind) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(primary), Ok(())) => Err(primary),
            (Ok(_), Err(fault)) => Err(fault),
            (Err(primary), Err(fault)) => Err(self.cleanup(primary, fault)),
        }
    }

    pub(super) fn faults(&self, faults: Faults) -> V {
        let primary = self.error(faults.primary);
        match faults.cleanup {
            Some(fault) => self.cleanup(primary, self.error(fault)),
            None => primary,
        }
    }
}

impl<V: Clone, O, A: Into<V>, D, E: Into<V>, M> super::interpreter::ErrorTransport
    for Transport<V, O, A, D, E, M>
{
    type Value = V;

    fn error(&self, fault: OwnerFault) -> V {
        self.error(fault)
    }

    fn message(&self, message: &str) -> V {
        (self.errors.new)(message).into()
    }

    fn with_cause(&self, fault: OwnerFault, primary: V) -> V {
        self.with_cause(fault, primary)
    }

    fn cleanup(&self, primary: V, cleanup: V) -> V {
        self.cleanup(primary, cleanup)
    }

    fn faults(&self, faults: Faults) -> V {
        self.faults(faults)
    }

    fn complete<T>(&self, result: Result<T, V>, unwind: Result<(), V>) -> Result<T, V> {
        self.complete(result, unwind)
    }
}
