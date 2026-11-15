use std::rc::Rc;

use css::resident_owner::OwnerFault;
use serial_test::serial;

use super::owner_native_tests::*;

#[test]
#[serial]
fn clear_preserves_open_resident_and_obsolete_clear_skips_unrelated_bank() -> Result<(), OwnerFault>
{
    // Given a live resident and unrelated open owner.
    let _fixture = Fixture::new()?;
    let a = create();
    let b = create();
    run(a);
    // When the resident is cleared and later an obsolete owner requests clear.
    ok(OWNER.clear(a));
    assert!(!ok(OWNER.disposed(a)));
    run(b);
    EVENTS.with_borrow_mut(Vec::clear);
    ok(OWNER.clear(a));
    // Then unrelated attribution survives with no reset or preservation.
    assert!(EVENTS.with_borrow(Vec::is_empty));
    ACCESS.with(|bank| {
        assert_eq!(bank.core.bank_owner(), Some(bank.owner(b)?));
        Ok(())
    })?;
    Ok(())
}

#[test]
#[serial]
fn active_disposal_defers_targeted_drain_until_run_unwinds() -> Result<(), OwnerFault> {
    // Given an admitted run with an active export.
    let _fixture = Fixture::new()?;
    let a = create();
    ok(OWNER.begin(a));
    ok(OWNER.configured());
    let ticket = ok(OWNER.enter(0));
    // When its callback disposes and queues clear while still inside admission.
    ok(OWNER.dispose(a));
    ok(OWNER.clear(a));
    // Then reset is deferred until exact export and run frames unwind.
    assert!(ok(OWNER.disposed(a)));
    fault(OWNER.leave(ticket), OwnerFault::ClosedOwner);
    fault(OWNER.finish(), OwnerFault::ClosedOwner);
    ok(OWNER.drain());
    ACCESS.with(|bank| {
        assert_eq!(bank.core.bank_owner(), None);
        Ok(())
    })?;
    assert!(!CALLBACKS.with_borrow(|callbacks| callbacks.contains_key(&a)));
    Ok(())
}

#[test]
#[serial]
fn capture_disposal_and_clear_are_drained_before_direct_clear_returns() -> Result<(), OwnerFault> {
    // Given capture that queues a targeted deferred clear while disposing itself.
    let _fixture = Fixture::new()?;
    let a = create();
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.insert(
            a,
            Rc::new(move || {
                let value = snapshot()?;
                ok(OWNER.dispose(a));
                ok(OWNER.clear(a));
                Ok(value)
            }),
        )
    });
    run(a);
    // When direct clear executes preservation and then mandatory drain.
    fault(OWNER.clear(a), OwnerFault::CaptureFailed);
    // Then its deferred reset already completed before any innocent next read.
    ACCESS.with(|bank| {
        assert_eq!(bank.core.bank_owner(), None);
        assert!(bank.capture.is_none());
        Ok(())
    })?;
    assert!(!ACCESS.export_active());
    run(create());
    Ok(())
}

#[test]
#[serial]
fn unrelated_disposal_removes_only_its_callback_without_faulting_active_run()
-> Result<(), OwnerFault> {
    // Given two owner registrations and one running owner.
    let _fixture = Fixture::new()?;
    let a = create();
    let b = create();
    ok(OWNER.begin(a));
    ok(OWNER.configured());
    // When the unrelated owner is disposed.
    ok(OWNER.dispose(b));
    // Then current publication stays admitted and only the unrelated callback is released.
    assert!(!ok(OWNER.disposed(a)));
    assert!(ok(OWNER.disposed(b)));
    assert!(
        CALLBACKS
            .with_borrow(|callbacks| callbacks.contains_key(&a) && !callbacks.contains_key(&b))
    );
    assert_eq!(ACCESS.check_fault(), Ok(()));
    ok(OWNER.finish());
    Ok(())
}

#[test]
#[serial]
fn invalid_clear_identity_never_resets_the_engine() -> Result<(), OwnerFault> {
    // Given no issued owners.
    let _fixture = Fixture::new()?;
    // When targeted clear receives an unissued id.
    fault(OWNER.clear(999), OwnerFault::InvalidOwner);
    // Then no engine reset or callback occurs.
    assert!(EVENTS.with_borrow(Vec::is_empty));
    Ok(())
}
