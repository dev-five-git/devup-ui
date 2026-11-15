use std::fmt::Display;

use css::resident_owner::OwnerFault;

pub(crate) fn message_error<V, E: Into<V>>(message: impl Display, new: fn(&str) -> E) -> V {
    new(&message.to_string()).into()
}

pub(crate) struct Errors<V, E, T> {
    pub new: fn(&str) -> E,
    pub transport: T,
    pub structural: fn(&T, OwnerFault) -> V,
    pub attach: fn(V) -> V,
    pub fault: fn() -> Result<(), OwnerFault>,
}

impl<V, E: Into<V>, T> Errors<V, E, T> {
    pub(crate) fn message(&self, message: impl Display) -> V {
        message_error(message, self.new)
    }

    pub(crate) fn extraction(&self, message: &str) -> V {
        match (self.fault)() {
            Ok(()) => crate::resolver::extraction_error(message, self.new, self.attach),
            Err(fault) => (self.attach)((self.structural)(&self.transport, fault)),
        }
    }

    pub(crate) fn check(&self) -> Result<(), V> {
        (self.fault)().map_err(|fault| (self.attach)((self.structural)(&self.transport, fault)))
    }
}
