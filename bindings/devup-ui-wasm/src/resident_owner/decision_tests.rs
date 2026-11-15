use css::resident_owner::{
    Counter, ExportKind, ExportPlan, OwnerCoordinator, OwnerFault, OwnerState, OwnerToken,
    PreservationPlan, ResumePlan, RunPlan, RunTicket,
};

use super::decision::{CaptureFailure, capture_failure, unwind};

#[rstest::rstest]
#[case(
    css::resident_owner::TransitionPhase::Preservation,
    OwnerFault::CaptureFailed
)]
#[case(
    css::resident_owner::TransitionPhase::Activation,
    OwnerFault::ActivationFailed
)]
#[case(
    css::resident_owner::TransitionPhase::Configuration,
    OwnerFault::ConfigurationFailed
)]
#[case(css::resident_owner::TransitionPhase::Clear, OwnerFault::CleanupFailed)]
fn phase_mapping_preserves_each_typed_failure(
    #[case] phase: css::resident_owner::TransitionPhase,
    #[case] fault: OwnerFault,
) {
    // Given each typed transition phase.
    // When the actual production mapping selects its failure.
    let actual = super::decision::phase_fault(phase);
    // Then no phase is confused with another phase's refusal.
    assert_eq!(actual, fault);
}

fn run(
    core: &mut OwnerCoordinator<&'static str>,
    owner: OwnerToken,
) -> Result<RunTicket, OwnerFault> {
    let configuration = match core.begin_run(owner)? {
        RunPlan::Activate(plan) => core.activation_completed(plan.ticket)?,
        RunPlan::Configure(ticket) => ticket,
        RunPlan::Preserve(_) => panic!("fixture must not require preservation"),
    };
    core.configuration_completed(configuration)
}

fn preserve(core: &mut OwnerCoordinator<&'static str>) -> Result<PreservationPlan, OwnerFault> {
    match core.begin_export(ExportKind::Mutate)? {
        ExportPlan::Preserve(plan) => Ok(plan),
        ExportPlan::Admitted(_) => panic!("fixture must have a live resident"),
    }
}

fn commit(
    core: &mut OwnerCoordinator<&'static str>,
    plan: &PreservationPlan,
) -> Result<(), OwnerFault> {
    match core.checkpoint_completed(plan.ticket, "outgoing checkpoint")? {
        ResumePlan::Export(ExportPlan::Admitted(ticket)) => core.end_export(ticket),
        ResumePlan::Export(ExportPlan::Preserve(_)) | ResumePlan::Run(_) | ResumePlan::Clear(_) => {
            panic!("fixture must resume the raw export")
        }
    }
}

#[test]
fn committed_capture_keeps_exact_exhaustion_and_outgoing_authority() -> Result<(), OwnerFault> {
    // Given real committed core checkpoint authority after the continuation frame ends.
    let mut core = OwnerCoordinator::new()?;
    let owner = core.create_owner()?;
    let ticket = run(&mut core, owner)?;
    core.end_run(ticket)?;
    let plan = preserve(&mut core)?;
    commit(&mut core, &plan)?;
    let fault = OwnerFault::Exhausted(Counter::Ticket);
    // When the binding receives a post-commit continuation-allocation failure.
    let failure = capture_failure(&mut core, &plan, fault);
    // Then it retains exhaustion without a stale abort, and restores the committed payload.
    match failure {
        CaptureFailure::Committed(actual) => assert_eq!(actual, fault),
        CaptureFailure::Aborted { .. } => panic!("committed capture was aborted again"),
    }
    match core.begin_run(owner)? {
        RunPlan::Activate(activation) => {
            let Some(saved) = activation.restore else {
                panic!("committed outgoing checkpoint missing")
            };
            assert_eq!(saved.stamp(), plan.stamp);
            assert_eq!(*saved.payload(), "outgoing checkpoint");
            let configured = core.activation_completed(activation.ticket)?;
            let ticket = core.configuration_completed(configured)?;
            core.end_run(ticket)?;
        }
        RunPlan::Preserve(_) | RunPlan::Configure(_) => panic!("saved owner must restore"),
    }
    Ok(())
}

#[test]
fn older_checkpoint_cannot_authorize_skipping_live_capture_abort() -> Result<(), OwnerFault> {
    // Given an older saved checkpoint and a newer uncaptured live mutation epoch.
    let mut core = OwnerCoordinator::new()?;
    let owner = core.create_owner()?;
    let ticket = run(&mut core, owner)?;
    core.end_run(ticket)?;
    let old = preserve(&mut core)?;
    commit(&mut core, &old)?;
    let ticket = run(&mut core, owner)?;
    core.end_run(ticket)?;
    let current = preserve(&mut core)?;
    assert_ne!(old.stamp, current.stamp);
    // When the live capture fails despite the older checkpoint's existence.
    let failure = capture_failure(&mut core, &current, OwnerFault::Exhausted(Counter::Ticket));
    // Then matching abort unwinds and faults the owner instead of reusing stale authority.
    match failure {
        CaptureFailure::Aborted { primary, unwind } => {
            assert_eq!(primary, OwnerFault::Exhausted(Counter::Ticket));
            assert_eq!(unwind, Err(OwnerFault::CaptureFailed));
        }
        CaptureFailure::Committed(_) => panic!("older checkpoint accepted for a newer epoch"),
    }
    assert_eq!(
        core.owner_state(owner)?,
        OwnerState::Faulted(OwnerFault::CaptureFailed)
    );
    let fresh = core.create_owner()?;
    let ticket = run(&mut core, fresh)?;
    core.end_run(ticket)
}

#[test]
fn distinct_completion_failure_survives_first_binding_refusal() {
    // Given a first binding refusal and a separate completion fault.
    let latched = Err(OwnerFault::ForeignEngine);
    let completion = Err(OwnerFault::ClosedOwner);
    // When both are materialized after unwind.
    let Err(failure) = unwind(completion, latched, None) else {
        panic!("two faults were lost")
    };
    // Then the first refusal stays primary and the distinct fault stays explicit.
    assert_eq!(failure.primary, OwnerFault::ForeignEngine);
    assert_eq!(failure.cleanup, Some(OwnerFault::ClosedOwner));
}

#[test]
fn expected_matching_abort_is_not_a_second_cleanup_fault() {
    // Given matching configuration abort's documented Err and a caught binding refusal.
    let result = Err(OwnerFault::ConfigurationFailed);
    let latched = Err(OwnerFault::ForeignEngine);
    // When the successful failure unwind is classified.
    let Err(failure) = unwind(result, latched, Some(OwnerFault::ConfigurationFailed)) else {
        panic!("refusal was lost")
    };
    // Then only the actual refusal is reported, not a fabricated second cleanup failure.
    assert_eq!(failure.primary, OwnerFault::ForeignEngine);
    assert_eq!(failure.cleanup, None);
}

#[test]
fn repeated_same_refusal_is_not_a_distinct_cleanup_fault() {
    // Given the same refusal retained by both authority layers.
    let refusal = Err(OwnerFault::Reentry);
    // When both unwind results are materialized.
    let Err(failure) = unwind(refusal, refusal, None) else {
        panic!("refusal was lost")
    };
    // Then the same fault is not duplicated as a separate cause.
    assert_eq!(failure.primary, OwnerFault::Reentry);
    assert_eq!(failure.cleanup, None);
}

#[test]
fn completion_without_latch_and_clean_unwind_preserve_their_outcomes() {
    // Given successful unwind or one unlatched failure.
    // When common unwind classification observes each outcome.
    let clean = unwind(Ok(()), Ok(()), None);
    let Err(failed) = unwind(Err(OwnerFault::CleanupFailed), Ok(()), None) else {
        panic!("cleanup fault was lost")
    };
    let Err(latched) = unwind(Ok(()), Err(OwnerFault::Reentry), None) else {
        panic!("binding fault was lost")
    };
    // Then success stays success and single failures acquire no invented cleanup cause.
    assert_eq!(clean, Ok(()));
    assert_eq!(failed.primary, OwnerFault::CleanupFailed);
    assert_eq!(failed.cleanup, None);
    assert_eq!(latched.primary, OwnerFault::Reentry);
    assert_eq!(latched.cleanup, None);
}
