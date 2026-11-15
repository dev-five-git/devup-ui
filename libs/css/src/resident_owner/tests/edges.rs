use super::*;

#[test]
fn old_checkpoint_cannot_recover_uncaptured_newer_allocations_after_capture_failure() -> TestResult
{
    // Given
    let (mut core, owner) = resident()?;
    let other = core.create_owner()?;
    let capture = takeover(&mut core, other)?;
    let plan = resume_run(core.checkpoint_completed(capture.ticket, vec![1])?);
    let configuration = core.activation_completed(plan.ticket)?;
    let run = core.configuration_completed(configuration)?;
    core.end_run(run)?;
    let capture = takeover(&mut core, owner)?;
    let plan = resume_run(core.checkpoint_completed(capture.ticket, vec![2])?);
    let old = plan.restore.ok_or(OwnerFault::CaptureFailed)?;
    let configuration = core.activation_completed(plan.ticket)?;
    let run = core.configuration_completed(configuration)?;
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    core.end_export(export)?;
    core.end_run(run)?;
    let capture = takeover(&mut core, other)?;
    // When
    let failure = core.abort_transition(
        TransitionTicket::Preservation(capture.ticket),
        TransitionPhase::Preservation,
    );
    // Then
    assert_eq!(failure, Err(OwnerFault::CaptureFailed));
    assert_eq!(old.payload(), &[1]);
    assert_eq!(
        core.checkpoint(owner).err(),
        Some(OwnerFault::CaptureFailed)
    );
    assert_eq!(core.begin_run(owner).err(), Some(OwnerFault::CaptureFailed));
    Ok(())
}

#[test]
fn deferred_clear_drains_obsolete_targets_without_resetting_the_active_owner_early() -> TestResult {
    // Given
    let (mut core, old) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let activation = resume_run(core.checkpoint_completed(capture.ticket, vec![7])?);
    let configuration = core.activation_completed(activation.ticket)?;
    let run = core.configuration_completed(configuration)?;
    core.dispose(old)?;
    core.dispose(incoming)?;
    assert_eq!(core.request_clear(old)?, ClearPlan::Deferred);
    assert_eq!(core.request_clear(incoming)?, ClearPlan::Deferred);
    assert_eq!(core.end_run(run), Err(OwnerFault::ClosedOwner));
    // When
    let plan = core.poll_clear()?;
    // Then
    let ClearPlan::Reset(reset) = plan else {
        panic!("skip obsolete target, reset current target")
    };
    assert_eq!(core.bank_owner(), Some(incoming));
    core.clear_completed(reset)?;
    assert_eq!(core.poll_clear()?, ClearPlan::Skipped);
    Ok(())
}

#[test]
fn disposal_during_activation_never_resurrects_incoming_and_keeps_outgoing_capture() -> TestResult {
    // Given
    let (mut core, old) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let plan = resume_run(core.checkpoint_completed(capture.ticket, vec![33])?);
    // When
    core.dispose(incoming)?;
    // Then
    assert_eq!(
        core.activation_completed(plan.ticket),
        Err(OwnerFault::ClosedOwner)
    );
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Activation(plan.ticket),
            TransitionPhase::Activation
        ),
        Err(OwnerFault::ClosedOwner)
    );
    assert_eq!(core.owner_state(incoming)?, OwnerState::Disposed);
    assert_eq!(
        core.checkpoint(old)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[33]
    );
    Ok(())
}

#[test]
fn caught_reentry_during_clear_requires_explicit_failure_unwind_and_retry() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    core.dispose(owner)?;
    let ClearPlan::Reset(reset) = core.request_clear(owner)? else {
        panic!("reset")
    };
    // When
    let refusal = core.begin_export(ExportKind::Read);
    // Then
    assert_eq!(refusal, Err(OwnerFault::Reentry));
    assert_eq!(core.clear_completed(reset), Err(OwnerFault::Reentry));
    assert_eq!(
        core.abort_transition(TransitionTicket::Clear(reset), TransitionPhase::Clear),
        Err(OwnerFault::Reentry)
    );
    let ClearPlan::Reset(retry) = core.poll_clear()? else {
        panic!("retry")
    };
    core.clear_completed(retry)
}

#[test]
fn invalid_owner_metadata_and_lifecycle_are_typed_refusals() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let unissued = OwnerToken {
        engine: core.engine_id(),
        sequence: 99,
    };
    // When
    let state = core.owner_state(unissued);
    // Then
    assert_eq!(state, Err(OwnerFault::InvalidOwner));
    assert_eq!(
        core.begin_run(unissued).err(),
        Some(OwnerFault::InvalidOwner)
    );
    assert_eq!(core.dispose(unissued), Err(OwnerFault::InvalidOwner));
    assert_eq!(core.request_clear(unissued), Err(OwnerFault::InvalidOwner));
    Ok(())
}

#[test]
fn wrong_preservation_context_is_refused_without_an_epoch_charge() -> TestResult {
    // Given
    let (mut source, _) = resident()?;
    let incoming = source.create_owner()?;
    let capture = takeover(&mut source, incoming)?;
    let mut target = Core::new()?;
    let kind = ExportKind::Preservation(capture.ticket, PreservationKind::Sheet);
    // When
    let refusal = target.begin_export(kind);
    // Then
    assert_eq!(refusal, Err(OwnerFault::WrongPhase));
    assert_eq!(target.epoch(), Epoch(0));
    let owner = target.create_owner()?;
    let run = start(&mut target, owner)?;
    assert_eq!(target.begin_export(kind), Err(OwnerFault::Reentry));
    assert_eq!(target.end_run(run), Err(OwnerFault::Reentry));
    Ok(())
}

#[test]
fn fault_format_preserves_the_typed_failure_category() {
    // Given
    let fault = OwnerFault::Exhausted(Counter::Epoch);
    // When
    let displayed = fault.to_string();
    // Then
    assert_eq!(displayed, "resident ownership fault: Exhausted(Epoch)");
    assert!(std::error::Error::source(&fault).is_none());
}
