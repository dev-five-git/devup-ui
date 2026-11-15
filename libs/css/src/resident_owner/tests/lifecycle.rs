use super::*;

#[test]
fn clear_of_open_resident_preserves_before_reset_and_restores_on_next_run() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let epoch = core.epoch();
    // When
    let ClearPlan::Preserve(capture) = core.request_clear(owner)? else {
        panic!("preserve")
    };
    // Then
    assert_eq!(core.epoch(), epoch);
    let ResumePlan::Clear(ClearPlan::Reset(reset)) =
        core.checkpoint_completed(capture.ticket, vec![17])?
    else {
        panic!("reset")
    };
    assert_eq!(core.bank_owner(), Some(owner));
    assert_eq!(core.resident_owner(), None);
    core.clear_completed(reset)?;
    assert_eq!(core.bank_owner(), None);
    let RunPlan::Activate(plan) = core.begin_run(owner)? else {
        panic!("restore")
    };
    assert_eq!(
        plan.restore.ok_or(OwnerFault::CaptureFailed)?.payload(),
        &[17]
    );
    Ok(())
}

#[test]
fn dispose_during_action_tombstones_now_but_clear_waits_for_export_and_run() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let run = start(&mut core, owner)?;
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    let epoch = core.epoch();
    // When
    core.dispose(owner)?;
    // Then
    assert_eq!(core.owner_state(owner)?, OwnerState::Disposed);
    assert_eq!(core.resident_owner(), None);
    assert_eq!(core.request_clear(owner)?, ClearPlan::Deferred);
    assert_eq!(core.poll_clear()?, ClearPlan::Deferred);
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.end_run(run), Err(OwnerFault::Reentry));
    assert_eq!(core.end_export(export), Err(OwnerFault::ClosedOwner));
    assert_eq!(core.end_run(run), Err(OwnerFault::ClosedOwner));
    assert_eq!(
        core.begin_export(ExportKind::Read),
        Err(OwnerFault::CleanupPending)
    );
    let ClearPlan::Reset(reset) = core.poll_clear()? else {
        panic!("deferred reset")
    };
    core.clear_completed(reset)?;
    assert_eq!(core.begin_run(owner).err(), Some(OwnerFault::ClosedOwner));
    assert_eq!(core.checkpoint(owner).err(), Some(OwnerFault::ClosedOwner));
    Ok(())
}

#[test]
fn disposed_old_owner_cannot_reset_a_new_owners_bank() -> TestResult {
    // Given
    let (mut core, old) = resident()?;
    let next = core.create_owner()?;
    let capture = takeover(&mut core, next)?;
    let activation = resume_run(core.checkpoint_completed(capture.ticket, vec![3])?);
    let configuration = core.activation_completed(activation.ticket)?;
    let run = core.configuration_completed(configuration)?;
    core.end_run(run)?;
    let epoch = core.epoch();
    // When
    core.dispose(old)?;
    let clear = core.request_clear(old)?;
    // Then
    assert_eq!(clear, ClearPlan::Skipped);
    assert_eq!(core.bank_owner(), Some(next));
    assert_eq!(core.resident_owner(), Some(next));
    assert_eq!(core.epoch(), epoch);
    core.dispose(old)?;
    assert_eq!(core.owner_state(old)?, OwnerState::Disposed);
    Ok(())
}

#[test]
fn configuration_disposal_can_abort_then_clear_between_exports() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let RunPlan::Configure(configuration) = core.begin_run(owner)? else {
        panic!("configure")
    };
    // When
    core.dispose(owner)?;
    // Then
    assert_eq!(
        core.configuration_completed(configuration),
        Err(OwnerFault::ClosedOwner)
    );
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Configuration(configuration),
            TransitionPhase::Configuration
        ),
        Err(OwnerFault::ClosedOwner)
    );
    let ClearPlan::Reset(reset) = core.request_clear(owner)? else {
        panic!("reset")
    };
    core.clear_completed(reset)?;
    assert_eq!(core.bank_owner(), None);
    assert_eq!(core.owner_state(owner)?, OwnerState::Disposed);
    Ok(())
}

#[test]
fn disposing_outgoing_capture_blocks_commit_but_retains_unwind_ticket() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    // When
    core.dispose(owner)?;
    // Then
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![42]).err(),
        Some(OwnerFault::ClosedOwner)
    );
    assert_eq!(core.request_clear(owner)?, ClearPlan::Deferred);
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Preservation(capture.ticket),
            TransitionPhase::Preservation
        ),
        Err(OwnerFault::ClosedOwner)
    );
    let ClearPlan::Reset(reset) = core.poll_clear()? else {
        panic!("reset")
    };
    core.clear_completed(reset)?;
    assert_eq!(core.owner_state(owner)?, OwnerState::Disposed);
    Ok(())
}

#[test]
fn disposing_incoming_capture_target_cannot_publish_its_action() -> TestResult {
    // Given
    let (mut core, outgoing) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    // When
    core.dispose(incoming)?;
    // Then
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![4]).err(),
        Some(OwnerFault::ClosedOwner)
    );
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Preservation(capture.ticket),
            TransitionPhase::Preservation
        ),
        Err(OwnerFault::ClosedOwner)
    );
    assert_eq!(core.bank_owner(), Some(outgoing));
    assert_eq!(core.request_clear(incoming)?, ClearPlan::Skipped);
    Ok(())
}

#[test]
fn clear_failure_retains_checkpoint_and_requires_retry_before_new_idle_work() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let ClearPlan::Preserve(capture) = core.request_clear(owner)? else {
        panic!("capture")
    };
    let ResumePlan::Clear(ClearPlan::Reset(reset)) =
        core.checkpoint_completed(capture.ticket, vec![22])?
    else {
        panic!("reset")
    };
    // When
    let failure = core.abort_transition(TransitionTicket::Clear(reset), TransitionPhase::Clear);
    // Then
    assert_eq!(failure, Err(OwnerFault::CleanupFailed));
    assert_eq!(
        core.begin_run(owner).err(),
        Some(OwnerFault::CleanupPending)
    );
    assert_eq!(
        core.checkpoint(owner)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[22]
    );
    let ClearPlan::Reset(retry) = core.poll_clear()? else {
        panic!("retry")
    };
    core.clear_completed(retry)?;
    assert_eq!(core.poll_clear()?, ClearPlan::Skipped);
    Ok(())
}

#[test]
fn idle_read_disposal_defers_reset_until_its_result_processing_unwinds() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let export = admitted(core.begin_export(ExportKind::Read)?);
    // When
    core.dispose(owner)?;
    // Then
    assert_eq!(core.request_clear(owner)?, ClearPlan::Deferred);
    assert_eq!(core.end_export(export), Err(OwnerFault::ClosedOwner));
    let ClearPlan::Reset(reset) = core.poll_clear()? else {
        panic!("reset")
    };
    core.clear_completed(reset)
}
