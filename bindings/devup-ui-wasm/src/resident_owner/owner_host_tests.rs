use std::rc::Rc;

use css::resident_owner::{OwnerFault, ResumePlan, TransitionPhase, TransitionTicket};
use rstest::rstest;
use serial_test::serial;

use super::{checkpoint::HostCheckpoint, error_native_tests::Value, owner_native_tests::*};

fn contains(error: &Value, original: &Value) -> bool {
    error == original
        || error.is_object()
            && (contains(&error.get("cause"), original)
                || contains(&error.get("cleanupCause"), original))
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn failed_capture_aborts_exact_ticket_and_preserves_thrown_identity(
    #[case] undefined: bool,
) -> Result<(), OwnerFault> {
    // Given a resident capture throwing an exact host reference or undefined.
    let _fixture = Fixture::new()?;
    let thrown = if undefined {
        Value::Undefined
    } else {
        Value::new()
    };
    let captured = thrown.clone();
    let a = ok(OWNER.create(Rc::new(move || Err(captured.clone()))));
    run(a);
    // When another owner requests preservation.
    let error = fault(OWNER.begin(create()), OwnerFault::CaptureFailed);
    // Then matching abort retains the original cause and releases the capture slot.
    assert_eq!(error.get("cause"), thrown);
    assert!(ok(OWNER.disposed(a)));
    ACCESS.with(|bank| {
        assert!(bank.capture.is_none());
        Ok(())
    })?;
    run(create());
    Ok(())
}

#[test]
#[serial]
fn missing_callback_faults_capture_without_reusing_an_older_snapshot() -> Result<(), OwnerFault> {
    // Given a live preservation ticket whose capture registration has disappeared.
    let _fixture = Fixture::new()?;
    let a = create();
    let plan = preservation(a)?;
    CALLBACKS.with_borrow_mut(|callbacks| callbacks.remove(&a));
    // When production executes the preservation plan.
    let error = fault(OWNER.preserve(plan), OwnerFault::CaptureFailed);
    // Then the missing callback is explicit and the owner becomes unusable.
    assert!(error.get("cause").get("message").as_string().is_some());
    assert!(ok(OWNER.disposed(a)));
    Ok(())
}

#[test]
#[serial]
fn missing_outgoing_transport_identity_cannot_start_capture() -> Result<(), OwnerFault> {
    // Given a genuine plan but no corresponding binding identity.
    let _fixture = Fixture::new()?;
    let a = create();
    let plan = preservation(a)?;
    ACCESS.with(|bank| {
        bank.owners.remove(&a);
        Ok(())
    })?;
    // When the interpreter resolves the plan's outgoing callback.
    fault(OWNER.preserve(plan), OwnerFault::InvalidOwner);
    // Then it has not opened a capture slot or called host work.
    ACCESS.with(|bank| {
        assert!(bank.capture.is_none());
        Ok(())
    })?;
    assert_eq!(EVENTS.with_borrow(Clone::clone), ["reset"]);
    Ok(())
}

#[test]
#[serial]
fn caught_capture_refusal_keeps_original_throw_and_distinct_cleanup() -> Result<(), OwnerFault> {
    // Given capture that catches a refusal and throws its own exception.
    let _fixture = Fixture::new()?;
    let thrown = Value::new();
    let original = thrown.clone();
    let a = ok(OWNER.create(Rc::new(move || {
        fault(OWNER.reject(), OwnerFault::ForeignEngine);
        Err(original.clone())
    })));
    run(a);
    // When takeover aborts and then unwinds the binding latch.
    let error = fault(OWNER.begin(create()), OwnerFault::CaptureFailed);
    // Then the complete error tree retains both authorities by identity/category.
    assert!(contains(&error, &thrown));
    assert!(error.property("cleanupCause").is_some());
    assert_eq!(ACCESS.check_fault(), Ok(()));
    Ok(())
}

#[test]
#[serial]
fn committed_capture_remains_authoritative_after_duplicate_completion() -> Result<(), OwnerFault> {
    // Given capture that completes the real core checkpoint before the outer completion.
    let _fixture = Fixture::new()?;
    let a = create();
    let plan = preservation(a)?;
    let ticket = plan.ticket;
    let stamp = plan.stamp;
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.insert(
            a,
            Rc::new(move || {
                let wire = parts().unwrap_or_else(|error| panic!("snapshot: {error}"));
                let payload = HostCheckpoint::parse(wire.each_ref().map(String::as_str))
                    .unwrap_or_else(|error| panic!("checkpoint: {error}"));
                ACCESS
                    .with(|bank| {
                        let ResumePlan::Export(css::resident_owner::ExportPlan::Admitted(export)) =
                            bank.core.checkpoint_completed(ticket, payload)?
                        else {
                            panic!("expected raw continuation")
                        };
                        bank.core.end_export(export)
                    })
                    .unwrap_or_else(|error| panic!("commit: {error:?}"));
                Ok(envelope(wire))
            }),
        )
    });
    // When the outer interpreter sees the real stale completion result.
    fault(OWNER.preserve(plan), OwnerFault::StaleTicket);
    // Then it skips a second abort and retains the exact committed stamp for restore.
    ACCESS.with(|bank| {
        assert_eq!(
            bank.core
                .checkpoint(bank.owner(a)?)?
                .map(|saved| saved.stamp()),
            Some(stamp)
        );
        Ok(())
    })?;
    run(a);
    Ok(())
}

#[test]
#[serial]
fn reentrant_disposal_prevents_capture_commit_before_publication() -> Result<(), OwnerFault> {
    // Given capture that disposes its outgoing owner after reading the wire.
    let _fixture = Fixture::new()?;
    let a = create();
    CALLBACKS.with_borrow_mut(|callbacks| {
        callbacks.insert(
            a,
            Rc::new(move || {
                let wire = snapshot()?;
                ok(OWNER.dispose(a));
                Ok(wire)
            }),
        )
    });
    run(a);
    // When takeover tries to commit the decoded payload.
    fault(OWNER.begin(create()), OwnerFault::CaptureFailed);
    // Then ClosedOwner was latched before commit and no callback survives disposal.
    assert!(ok(OWNER.disposed(a)));
    assert!(!CALLBACKS.with_borrow(|callbacks| callbacks.contains_key(&a)));
    assert!(EVENTS.with_borrow(|events| events.contains(&"retain".into())));
    Ok(())
}

#[test]
#[serial]
fn bank_initialization_failure_during_callback_is_not_hidden_by_unwind() -> Result<(), OwnerFault> {
    // Given capture that exposes an actual typed failed-bank state after producing wire.
    let _fixture = Fixture::new()?;
    let a = ok(OWNER.create(Rc::new(|| {
        let wire = snapshot()?;
        *STORE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(OwnerFault::ForeignEngine);
        Ok(wire)
    })));
    run(a);
    // When production reaches completion and capture-slot unwind through Access.
    let error = fault(OWNER.begin(create()), OwnerFault::ForeignEngine);
    // Then completion and unwind faults are both explicit, not a fabricated success.
    assert!(error.property("cleanupCause").is_some());
    Ok(())
}

#[test]
#[serial]
fn abort_classifier_maps_impossible_success_to_explicit_wrong_phase_cleanup()
-> Result<(), OwnerFault> {
    // Given the public Result contract of the abort classifier and a primary throw.
    let _fixture = Fixture::new()?;
    let thrown = Value::new();
    // When the classifier receives an invalid success outcome (not a replaced interpreter).
    let error = OWNER.aborted(Ok(()), TransitionPhase::Clear, thrown.clone());
    // Then the guard retains the phase failure and an explicit WrongPhase cleanup.
    assert!(contains(&error, &thrown));
    assert_eq!(
        super::error_native_tests::TRANSPORT
            .details(&error.get("cleanupCause"))
            .at(0),
        Value::text("WrongPhase")
    );
    Ok(())
}

#[test]
#[serial]
fn stale_abort_retains_primary_and_reports_separate_ticket_fault() -> Result<(), OwnerFault> {
    // Given an activation ticket already completed into configuration.
    let _fixture = Fixture::new()?;
    let a = create();
    let ticket = ACCESS.with(|bank| {
        let css::resident_owner::RunPlan::Activate(plan) = bank.core.begin_run(bank.owner(a)?)?
        else {
            panic!("activation required")
        };
        bank.core.activation_completed(plan.ticket)?;
        Ok(plan.ticket)
    })?;
    let primary = Value::new();
    // When the actual core rejects the old ticket during abort.
    let error = OWNER.abort(
        TransitionTicket::Activation(ticket),
        TransitionPhase::Activation,
        primary.clone(),
    );
    // Then the stale fault cannot replace the original primary.
    assert!(contains(&error, &primary));
    assert!(error.property("cleanupCause").is_some());
    Ok(())
}
