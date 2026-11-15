use std::rc::Rc;

use css::resident_owner::OwnerFault;
use serial_test::serial;

use super::{error_native_tests::CREATED, owner_native_tests::*};

#[test]
#[serial]
fn same_resident_reconfigures_without_capture_reset_restore_or_error() -> Result<(), OwnerFault> {
    // Given a successfully configured resident.
    let _fixture = Fixture::new()?;
    let id = create();
    run(id);
    EVENTS.with_borrow_mut(Vec::clear);
    CREATED.set(0);
    // When it runs again through the production interpreter.
    run(id);
    // Then no preservation, reset, restore or Error work occurred.
    assert!(EVENTS.with_borrow(Vec::is_empty));
    assert_eq!(CREATED.get(), 0);
    assert!(!ACCESS.export_active());
    Ok(())
}

#[test]
#[serial]
fn takeover_restores_real_five_family_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    // Given two owners and nonempty real A engine state.
    let _fixture = Fixture::new()?;
    let a = create();
    let b = create();
    run(a);
    let expected = crate::owner_codec_remaining_tests::install_owner("A")?;
    run(b);
    crate::owner_codec_remaining_tests::install_owner("B")?;
    EVENTS.with_borrow_mut(Vec::clear);
    // When A takes back the resident bank.
    run(a);
    // Then each family is exact and reset precedes restore after five capture reads.
    assert_eq!(parts()?, expected);
    assert_eq!(
        EVENTS.with_borrow(Clone::clone),
        [
            "sheet",
            "classes",
            "files",
            "canonical",
            "reservation",
            "reset",
            "restore"
        ]
    );
    Ok(())
}

#[test]
#[serial]
fn raw_mutation_preserves_resident_and_matching_ticket_unwinds() -> Result<(), OwnerFault> {
    // Given a recoverable resident.
    let _fixture = Fixture::new()?;
    let id = create();
    run(id);
    // When a raw export is admitted and completed.
    let export = ok(OWNER.enter(0));
    assert!(ACCESS.export_active());
    ok(OWNER.leave(export));
    // Then the checkpoint has committed and the raw ticket is released.
    ACCESS.with(|bank| {
        assert!(bank.core.checkpoint(bank.owner(id)?)?.is_some());
        Ok(())
    })?;
    assert!(!ACCESS.export_active());
    Ok(())
}

#[test]
#[serial]
fn caught_nested_refusal_remains_primary_until_finish() -> Result<(), OwnerFault> {
    // Given an admitted managed run and export.
    let _fixture = Fixture::new()?;
    let id = create();
    ok(OWNER.begin(id));
    ok(OWNER.configured());
    let export = ok(OWNER.enter(0));
    // When a nested callback catches a foreign refusal.
    fault(OWNER.reject(), OwnerFault::ForeignEngine);
    // Then publication, leave and finish all retain the first refusal.
    assert_eq!(ACCESS.check_fault(), Err(OwnerFault::ForeignEngine));
    fault(OWNER.leave(export), OwnerFault::ForeignEngine);
    fault(OWNER.finish(), OwnerFault::ForeignEngine);
    assert_eq!(ACCESS.check_fault(), Ok(()));
    Ok(())
}

#[test]
#[serial]
fn wrong_export_ticket_cannot_release_the_matching_export() -> Result<(), OwnerFault> {
    // Given an idle raw export ticket.
    let _fixture = Fixture::new()?;
    let export = ok(OWNER.enter(28));
    // When completion presents another transport identity.
    fault(OWNER.leave(export + 1), OwnerFault::StaleTicket);
    // Then the real ticket remains active and can complete normally.
    assert!(ACCESS.export_active());
    ok(OWNER.leave(export));
    fault(OWNER.leave(export), OwnerFault::StaleTicket);
    Ok(())
}

#[test]
#[serial]
fn missing_run_and_configuration_failures_keep_their_categories() -> Result<(), OwnerFault> {
    // Given an idle bank and an owner whose configuration never completes.
    let _fixture = Fixture::new()?;
    fault(OWNER.configured(), OwnerFault::WrongPhase);
    fault(OWNER.finish(), OwnerFault::WrongPhase);
    let id = create();
    ok(OWNER.begin(id));
    // When finish aborts that exact configuration frame.
    fault(OWNER.finish(), OwnerFault::ConfigurationFailed);
    // Then the owner is faulted and the bank can admit a replacement.
    assert!(ok(OWNER.disposed(id)));
    run(create());
    Ok(())
}

#[test]
#[serial]
fn wrong_configuration_completion_does_not_replace_a_running_ticket() -> Result<(), OwnerFault> {
    // Given a running owner.
    let _fixture = Fixture::new()?;
    let id = create();
    ok(OWNER.begin(id));
    ok(OWNER.configured());
    // When configuration completion is repeated.
    fault(OWNER.configured(), OwnerFault::WrongPhase);
    // Then the matching run still finishes normally.
    ok(OWNER.finish());
    Ok(())
}

#[test]
#[serial]
fn missing_ids_and_invalid_opcodes_do_not_admit_callbacks() -> Result<(), OwnerFault> {
    // Given an empty owner registry.
    let _fixture = Fixture::new()?;
    // When scalar admission sees missing ids and an invalid fixed inventory opcode.
    fault(OWNER.begin(91), OwnerFault::InvalidOwner);
    fault(OWNER.enter(36), OwnerFault::WrongPhase);
    fault(OWNER.dispose(91), OwnerFault::InvalidOwner);
    fault(OWNER.disposed(91), OwnerFault::InvalidOwner);
    // Then no callback, mutation or Error diagnostic getter is consulted.
    assert!(CALLBACKS.with_borrow(BTreeMap::is_empty));
    assert!(EVENTS.with_borrow(Vec::is_empty));
    Ok(())
}

use std::collections::BTreeMap;

#[test]
#[serial]
fn capture_exports_are_allowlisted_and_callback_can_remove_itself() -> Result<(), OwnerFault> {
    // Given capture that reenters each real snapshot export and releases its own handle.
    let _fixture = Fixture::new()?;
    let a = create();
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.insert(
            a,
            Rc::new(move || {
                for opcode in 23..=27 {
                    let ticket = ok(OWNER.enter(opcode));
                    ok(OWNER.leave(ticket));
                }
                CALLBACKS.with_borrow_mut(|callbacks| callbacks.remove(&a));
                snapshot()
            }),
        )
    });
    run(a);
    // When takeover invokes that cloned callback without either borrow held.
    run(create());
    // Then capture commits even though callback storage was removed in flight.
    assert!(!CALLBACKS.with_borrow(|callbacks| callbacks.contains_key(&a)));
    ACCESS.with(|bank| {
        assert!(bank.capture.is_none());
        assert!(bank.core.checkpoint(bank.owner(a)?)?.is_some());
        Ok(())
    })?;
    Ok(())
}
