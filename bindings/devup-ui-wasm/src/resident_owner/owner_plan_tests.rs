use css::resident_owner::{
    ClearPlan, ExportKind, ExportPlan, OwnerFault, PreservationPlan, RunPlan,
};
use rstest::rstest;
use serial_test::serial;

use super::owner_native_tests::*;

#[derive(Clone, Copy)]
enum Request {
    Run,
    Export,
    Clear,
}

fn plan(request: Request, resident: u64, incoming: u64) -> Result<PreservationPlan, OwnerFault> {
    ACCESS.with(|bank| match request {
        Request::Run => match bank.core.begin_run(bank.owner(incoming)?)? {
            RunPlan::Preserve(plan) => Ok(plan),
            RunPlan::Activate(_) | RunPlan::Configure(_) => panic!("expected preservation"),
        },
        Request::Export => match bank.core.begin_export(ExportKind::Mutate)? {
            ExportPlan::Preserve(plan) => Ok(plan),
            ExportPlan::Admitted(_) => panic!("expected preservation"),
        },
        Request::Clear => match bank.core.request_clear(bank.owner(resident)?)? {
            ClearPlan::Preserve(plan) => Ok(plan),
            ClearPlan::Reset(_) | ClearPlan::Deferred | ClearPlan::Skipped => {
                panic!("expected preservation")
            }
        },
    })
}

#[rstest]
#[case(Request::Run, Request::Export)]
#[case(Request::Run, Request::Clear)]
#[case(Request::Export, Request::Run)]
#[case(Request::Export, Request::Clear)]
#[case(Request::Clear, Request::Run)]
#[case(Request::Clear, Request::Export)]
#[serial]
fn wrong_interpreter_rejects_real_core_continuation_after_capture(
    #[case] interpreter: Request,
    #[case] pending: Request,
) -> Result<(), OwnerFault> {
    // Given a genuine core preservation plan with another continuation kind.
    let _fixture = Fixture::new()?;
    let a = create();
    let b = create();
    run(a);
    let plan = plan(pending, a, b)?;
    // When a typed invalid outer plan sends it through the wrong production interpreter.
    let result = match interpreter {
        Request::Run => OWNER.configure(RunPlan::Preserve(plan)),
        Request::Export => OWNER.admitted(ExportPlan::Preserve(plan), 999).map(|_| ()),
        Request::Clear => OWNER.execute(ClearPlan::Preserve(plan)).map(|_| ()),
    };
    // Then actual capture/commit ran but the mismatched continuation never executes.
    fault(result, OwnerFault::WrongPhase);
    assert_eq!(
        EVENTS.with_borrow(|events| events.iter().filter(|event| *event == "reset").count()),
        1
    );
    ACCESS.with(|bank| {
        assert!(bank.capture.is_none());
        assert!(bank.core.checkpoint(bank.owner(a)?)?.is_some());
        Ok(())
    })?;
    Ok(())
}

#[test]
#[serial]
fn stale_activation_performs_real_reset_then_reports_abort_faults() -> Result<(), OwnerFault> {
    // Given an activation plan whose real core ticket has already advanced.
    let _fixture = Fixture::new()?;
    let a = create();
    let plan = ACCESS.with(|bank| {
        let RunPlan::Activate(plan) = bank.core.begin_run(bank.owner(a)?)? else {
            panic!("expected activation")
        };
        bank.core.activation_completed(plan.ticket)?;
        Ok(plan)
    })?;
    // When production interprets that stale source plan.
    let error = fault(
        OWNER.configure(RunPlan::Activate(plan)),
        OwnerFault::ActivationFailed,
    );
    // Then the real infallible reset occurred and the separate stale abort stays explicit.
    assert_eq!(EVENTS.with_borrow(Clone::clone), ["reset"]);
    assert!(error.property("cleanupCause").is_some());
    Ok(())
}

#[test]
#[serial]
fn stale_clear_performs_real_reset_then_reports_exact_abort_failure() -> Result<(), OwnerFault> {
    // Given a real targeted reset ticket already completed by its owner.
    let _fixture = Fixture::new()?;
    let a = create();
    run(a);
    ok(OWNER.dispose(a));
    let ticket = ACCESS.with(|bank| {
        let ClearPlan::Reset(ticket) = bank.core.request_clear(bank.owner(a)?)? else {
            panic!("expected reset")
        };
        bank.core.clear_completed(ticket)?;
        Ok(ticket)
    })?;
    EVENTS.with_borrow_mut(Vec::clear);
    // When production interprets the stale typed clear plan.
    let error = fault(
        OWNER.execute(ClearPlan::Reset(ticket)),
        OwnerFault::CleanupFailed,
    );
    // Then it does not invent fallible reset/restore or discard the stale-ticket diagnostic.
    assert_eq!(EVENTS.with_borrow(Clone::clone), ["reset"]);
    assert!(error.property("cleanupCause").is_some());
    Ok(())
}

#[test]
#[serial]
fn failed_store_refuses_configuration_and_export_publication() -> Result<(), OwnerFault> {
    // Given a real configuration ticket and raw export admission from independent setup.
    let _fixture = Fixture::new()?;
    let a = create();
    let ticket = ACCESS.with(|bank| {
        let RunPlan::Activate(plan) = bank.core.begin_run(bank.owner(a)?)? else {
            panic!("expected activation")
        };
        bank.core.activation_completed(plan.ticket)
    })?;
    *STORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(OwnerFault::ForeignEngine);
    // When the interpreter attempts to publish its configuration frame.
    fault(
        OWNER.configure(RunPlan::Configure(ticket)),
        OwnerFault::ForeignEngine,
    );
    // Then no engine work happens on the direct configuration path.
    assert!(EVENTS.with_borrow(Vec::is_empty));
    Ok(())
}

#[test]
#[serial]
fn failed_store_cannot_publish_a_genuine_export_ticket() -> Result<(), OwnerFault> {
    // Given an actual admitted ticket before a typed store failure.
    let _fixture = Fixture::new()?;
    let plan = ACCESS.with(|bank| bank.core.begin_export(ExportKind::Read))?;
    *STORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(OwnerFault::ForeignEngine);
    // When publication executes in the shared production interpreter.
    fault(OWNER.admitted(plan, 4), OwnerFault::ForeignEngine);
    // Then the failed bank does not acquire a transport export slot.
    assert!(!ACCESS.export_active());
    Ok(())
}

#[test]
fn fixed_export_inventory_classifies_all_36_scalar_operations() {
    // Given all fixed public preconversion opcodes and no capture authorization.
    // When production classifies each opcode.
    let kinds: Vec<_> = (0..36)
        .map(|opcode| super::protocol::kind(opcode, None))
        .collect();
    // Then mutation/read boundaries match the preserved public inventory.
    assert_eq!(&kinds[..23], vec![Ok(ExportKind::Mutate); 23]);
    assert_eq!(&kinds[23..], vec![Ok(ExportKind::Read); 13]);
    assert_eq!(
        super::protocol::kind(u32::MAX, None),
        Err(OwnerFault::WrongPhase)
    );
}
