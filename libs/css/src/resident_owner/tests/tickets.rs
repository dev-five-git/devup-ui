use super::*;

#[test]
fn foreign_owners_are_rejected_even_when_numeric_owner_sequences_match() -> TestResult {
    // Given
    let mut left = Core::new()?;
    let mut right = Core::new()?;
    let local = left.create_owner()?;
    let foreign = right.create_owner()?;
    // When
    let admission = left.begin_run(foreign).err();
    // Then
    assert_eq!(local.sequence, foreign.sequence);
    assert_ne!(left.engine_id(), right.engine_id());
    assert_eq!(admission, Some(OwnerFault::ForeignEngine));
    assert_eq!(left.owner_state(foreign), Err(OwnerFault::ForeignEngine));
    assert_eq!(
        left.checkpoint(foreign).err(),
        Some(OwnerFault::ForeignEngine)
    );
    assert_eq!(left.dispose(foreign), Err(OwnerFault::ForeignEngine));
    assert_eq!(left.request_clear(foreign), Err(OwnerFault::ForeignEngine));
    assert_eq!(left.owner_state(local)?, OwnerState::Open);
    Ok(())
}

#[test]
fn foreign_export_ticket_cannot_release_a_local_read_guard() -> TestResult {
    // Given
    let mut left = Core::new()?;
    let mut right = Core::new()?;
    let local = admitted(left.begin_export(ExportKind::Read)?);
    let foreign = admitted(right.begin_export(ExportKind::Read)?);
    // When
    let refusal = left.end_export(foreign);
    // Then
    assert_eq!(refusal, Err(OwnerFault::ForeignEngine));
    assert_eq!(
        left.begin_export(ExportKind::Mutate),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(left.end_export(local), Err(OwnerFault::ForeignEngine));
    right.end_export(foreign)
}

#[test]
fn stale_run_cannot_end_a_new_run_and_refusal_latches_on_the_new_run() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let old = start(&mut core, owner)?;
    core.end_run(old)?;
    let current = start(&mut core, owner)?;
    // When
    let refusal = core.end_run(old);
    // Then
    assert_eq!(refusal, Err(OwnerFault::StaleTicket));
    assert_eq!(core.end_run(current), Err(OwnerFault::StaleTicket));
    assert_eq!(core.resident_owner(), Some(owner));
    Ok(())
}

#[test]
fn stale_capture_cannot_replace_a_completed_immutable_checkpoint() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let activation = resume_run(core.checkpoint_completed(capture.ticket, vec![6])?);
    let saved = core.checkpoint(owner)?.ok_or(OwnerFault::CaptureFailed)?;
    // When
    let refusal = core.checkpoint_completed(capture.ticket, vec![99]).err();
    // Then
    assert_eq!(refusal, Some(OwnerFault::StaleTicket));
    assert_eq!(saved.payload(), &[6]);
    assert_eq!(
        core.checkpoint(owner)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[6]
    );
    assert_eq!(
        core.activation_completed(activation.ticket),
        Err(OwnerFault::StaleTicket)
    );
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Activation(activation.ticket),
            TransitionPhase::Activation
        ),
        Err(OwnerFault::StaleTicket)
    );
    Ok(())
}

#[test]
fn preservation_run_reentry_is_refused_before_epoch_or_attribution_changes() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let epoch = core.epoch();
    // When
    let same = core.begin_run(owner).err();
    let different = core.begin_run(incoming).err();
    // Then
    assert_eq!(
        (same, different),
        (Some(OwnerFault::Reentry), Some(OwnerFault::Reentry))
    );
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.bank_owner(), Some(owner));
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![]).err(),
        Some(OwnerFault::Reentry)
    );
    Ok(())
}

#[test]
fn capture_completion_cannot_overtake_an_allowlisted_export_frame() -> TestResult {
    // Given
    let (mut core, _) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let export = admitted(core.begin_export(ExportKind::Preservation(
        capture.ticket,
        PreservationKind::Sheet,
    ))?);
    // When
    let completion = core.checkpoint_completed(capture.ticket, vec![]).err();
    // Then
    assert_eq!(completion, Some(OwnerFault::Reentry));
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Preservation(capture.ticket),
            TransitionPhase::Preservation
        ),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(core.end_export(export), Err(OwnerFault::Reentry));
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Preservation(capture.ticket),
            TransitionPhase::Preservation
        ),
        Err(OwnerFault::Reentry)
    );
    Ok(())
}

#[test]
fn preservation_export_requires_its_originating_capture_ticket() -> TestResult {
    // Given
    let (mut left, _) = resident()?;
    let next = left.create_owner()?;
    let local = takeover(&mut left, next)?;
    let (mut right, _) = resident()?;
    let next = right.create_owner()?;
    let foreign = takeover(&mut right, next)?;
    // When
    let refusal = left.begin_export(ExportKind::Preservation(
        foreign.ticket,
        PreservationKind::FileMap,
    ));
    // Then
    assert_eq!(refusal, Err(OwnerFault::ForeignEngine));
    assert_eq!(
        left.checkpoint_completed(local.ticket, vec![]).err(),
        Some(OwnerFault::ForeignEngine)
    );
    Ok(())
}

#[test]
fn wrong_abort_phase_does_not_release_activation_or_admit_action() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let owner = core.create_owner()?;
    let RunPlan::Activate(plan) = core.begin_run(owner)? else {
        panic!("activation")
    };
    // When
    let refusal = core.abort_transition(
        TransitionTicket::Activation(plan.ticket),
        TransitionPhase::Configuration,
    );
    // Then
    assert_eq!(refusal, Err(OwnerFault::WrongPhase));
    assert_eq!(
        core.activation_completed(plan.ticket),
        Err(OwnerFault::WrongPhase)
    );
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Activation(plan.ticket),
            TransitionPhase::Activation
        ),
        Err(OwnerFault::WrongPhase)
    );
    assert_eq!(core.resident_owner(), None);
    Ok(())
}
