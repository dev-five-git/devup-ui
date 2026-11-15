use std::sync::{LazyLock, Mutex};

use css::resident_owner::OwnerFault;
use serial_test::serial;

use super::{
    access::{Access, Store},
    bank::Bank,
    owner_native_tests::*,
};

struct RootBankGuard(Option<Result<Bank, OwnerFault>>);

impl RootBankGuard {
    fn new() -> Result<Self, OwnerFault> {
        let replacement = Bank::new()?;
        let mut bank = super::BANK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(Self(Some(std::mem::replace(&mut *bank, Ok(replacement)))))
    }
}

impl Drop for RootBankGuard {
    fn drop(&mut self) {
        if let Some(original) = self.0.take() {
            *super::BANK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = original;
        }
    }
}

#[test]
#[serial]
fn failed_initialization_refuses_access_and_reports_no_active_export() {
    // Given a real store containing a typed initialization failure.
    let store: Store = LazyLock::new(|| Mutex::new(Err(OwnerFault::ForeignEngine)));
    let access = Access { store: &store };
    // When root access guards observe it.
    let fault = access.check_fault();
    let active = access.export_active();
    // Then the exact initialization fault survives and no export is invented.
    assert_eq!(fault, Err(OwnerFault::ForeignEngine));
    assert!(!active);
}

#[test]
#[serial]
fn poisoned_mutex_recovers_the_owned_bank_without_holding_host_work() -> Result<(), OwnerFault> {
    // Given a poisoned but intact ordinary Rust bank store.
    let store: Store = LazyLock::new(|| Mutex::new(Bank::new()));
    let caught = std::panic::catch_unwind(|| {
        let _guard = store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        panic!("test poison");
    });
    assert!(caught.is_err());
    let access = Access { store: &store };
    // When the production Access helper recovers that data.
    let id = access.with(Bank::create)?;
    // Then the issued identity belongs to the same recovered bank.
    assert!(access.with(|bank| bank.owner(id)).is_ok());
    assert_eq!(access.check_fault(), Ok(()));
    Ok(())
}

#[test]
#[serial]
fn root_publication_guards_share_the_native_access_implementation() -> Result<(), OwnerFault> {
    // Given the real root bank at a quiescent test boundary.
    let _state = crate::TestStateGuard::new();
    // When the production root wrappers check their actual store.
    let checked = super::check_fault();
    let active = super::export_active();
    // Then default publication is admitted with no active raw export.
    assert_eq!(checked, Ok(()));
    assert!(!active);
    Ok(())
}

#[test]
#[serial]
fn root_guards_observe_real_active_ticket_and_first_binding_latch() -> Result<(), OwnerFault> {
    // Given the real root store with a recoverable test-scoped bank replacement.
    let _state = crate::TestStateGuard::new();
    let _bank = RootBankGuard::new()?;
    let access = Access {
        store: &super::BANK,
    };
    access.with(|bank| {
        let (css::resident_owner::ExportPlan::Admitted(ticket), id) = bank.enter_export(0)? else {
            panic!("idle mutation must admit")
        };
        bank.export = Some((id, ticket));
        assert_eq!(
            bank.latch::<()>(Err(OwnerFault::ForeignEngine)),
            Err(OwnerFault::ForeignEngine)
        );
        Ok(())
    })?;
    // When the actual root wrappers enforce publication and active-export scope.
    let checked = super::check_fault();
    let active = super::export_active();
    // Then neither a constant-OK guard nor a disconnected store could pass.
    assert_eq!(checked, Err(OwnerFault::ForeignEngine));
    assert!(active);
    Ok(())
}

#[test]
#[serial]
fn typed_failed_store_rejects_every_interpreter_entry_without_host_work() -> Result<(), OwnerFault>
{
    // Given failed bank state, not a preselected interpreter outcome.
    let _fixture = Fixture::new()?;
    *STORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(OwnerFault::ForeignEngine);
    // When actual entry methods access that failed store.
    fault(
        OWNER.create(std::rc::Rc::new(snapshot)),
        OwnerFault::ForeignEngine,
    );
    fault(OWNER.reject(), OwnerFault::ForeignEngine);
    fault(OWNER.begin(1), OwnerFault::ForeignEngine);
    fault(OWNER.configured(), OwnerFault::ForeignEngine);
    fault(OWNER.finish(), OwnerFault::ForeignEngine);
    fault(OWNER.enter(0), OwnerFault::ForeignEngine);
    fault(OWNER.leave(1), OwnerFault::ForeignEngine);
    fault(OWNER.dispose(1), OwnerFault::ForeignEngine);
    fault(OWNER.disposed(1), OwnerFault::ForeignEngine);
    fault(OWNER.clear(1), OwnerFault::ForeignEngine);
    fault(OWNER.drain(), OwnerFault::ForeignEngine);
    // Then all guards agree and no engine/capture/context callback ran.
    assert!(EVENTS.with_borrow(Vec::is_empty));
    Ok(())
}
