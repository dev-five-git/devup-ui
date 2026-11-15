use std::sync::{LazyLock, Mutex};

use css::resident_owner::OwnerFault;

use super::bank::Bank;

pub(super) type Store = LazyLock<Mutex<Result<Bank, OwnerFault>>>;

#[derive(Clone, Copy)]
pub(super) struct Access<'a> {
    pub store: &'a Store,
}

impl Access<'_> {
    pub fn with<T>(
        &self,
        action: impl FnOnce(&mut Bank) -> Result<T, OwnerFault>,
    ) -> Result<T, OwnerFault> {
        let mut guard = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match guard.as_mut() {
            Ok(bank) => action(bank),
            Err(fault) => Err(*fault),
        }
    }

    pub fn check_fault(&self) -> Result<(), OwnerFault> {
        self.with(|bank| bank.check_fault())
    }

    pub fn export_active(&self) -> bool {
        self.with(|bank| Ok(bank.export.is_some())).unwrap_or(false)
    }
}
