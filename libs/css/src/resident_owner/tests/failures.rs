use super::*;

#[test]
fn capture_failure_blocks_takeover_and_discards_older_checkpoint_authority() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let epoch = core.epoch();
    // When
    let failure = core.abort_transition(
        TransitionTicket::Preservation(capture.ticket),
        TransitionPhase::Preservation,
    );
    // Then
    assert_eq!(failure, Err(OwnerFault::CaptureFailed));
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.bank_owner(), Some(owner));
    assert_eq!(core.resident_owner(), None);
    assert_eq!(
        core.checkpoint(owner).err(),
        Some(OwnerFault::CaptureFailed)
    );
    assert_eq!(core.begin_run(owner).err(), Some(OwnerFault::CaptureFailed));
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![99]).err(),
        Some(OwnerFault::StaleTicket)
    );
    assert!(matches!(core.begin_run(incoming)?, RunPlan::Activate(_)));
    Ok(())
}

#[test]
fn activation_abort_retains_completed_outgoing_checkpoint_without_action() -> TestResult {
    // Given
    let (mut core, outgoing) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let activation = resume_run(core.checkpoint_completed(capture.ticket, vec![5, 8])?);
    // When
    let failure = core.abort_transition(
        TransitionTicket::Activation(activation.ticket),
        TransitionPhase::Activation,
    );
    // Then
    assert_eq!(failure, Err(OwnerFault::ActivationFailed));
    assert_eq!(core.bank_owner(), Some(incoming));
    assert_eq!(core.resident_owner(), None);
    assert_eq!(
        core.checkpoint(outgoing)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[5, 8]
    );
    assert_eq!(
        core.activation_completed(activation.ticket),
        Err(OwnerFault::StaleTicket)
    );
    assert_eq!(
        core.begin_run(incoming).err(),
        Some(OwnerFault::ActivationFailed)
    );
    let RunPlan::Activate(recovery) = core.begin_run(outgoing)? else {
        panic!("restore required")
    };
    assert_eq!(
        recovery.restore.ok_or(OwnerFault::CaptureFailed)?.payload(),
        &[5, 8]
    );
    Ok(())
}

#[test]
fn configuration_abort_after_partial_mutation_does_not_publish_residency() -> TestResult {
    // Given
    let (mut core, outgoing) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let activation = resume_run(core.checkpoint_completed(capture.ticket, vec![13])?);
    let configuration = core.activation_completed(activation.ticket)?;
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    core.end_export(export)?;
    // When
    let failure = core.abort_transition(
        TransitionTicket::Configuration(configuration),
        TransitionPhase::Configuration,
    );
    // Then
    assert_eq!(failure, Err(OwnerFault::ConfigurationFailed));
    assert_eq!(core.resident_owner(), None);
    assert_eq!(core.bank_owner(), Some(incoming));
    assert_eq!(
        core.configuration_completed(configuration),
        Err(OwnerFault::StaleTicket)
    );
    assert_eq!(
        core.checkpoint(outgoing)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[13]
    );
    Ok(())
}

#[test]
fn unmanaged_capture_failure_never_admits_the_destructive_export() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let ExportPlan::Preserve(capture) = core.begin_export(ExportKind::Mutate)? else {
        panic!("capture")
    };
    let epoch = core.epoch();
    // When
    let failure = core.abort_transition(
        TransitionTicket::Preservation(capture.ticket),
        TransitionPhase::Preservation,
    );
    // Then
    assert_eq!(failure, Err(OwnerFault::CaptureFailed));
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.bank_owner(), Some(owner));
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![]).err(),
        Some(OwnerFault::StaleTicket)
    );
    Ok(())
}

#[test]
fn reentry_during_activation_blocks_completion_until_explicit_abort() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let owner = core.create_owner()?;
    let RunPlan::Activate(plan) = core.begin_run(owner)? else {
        panic!("activation")
    };
    let epoch = core.epoch();
    // When
    let read = core.begin_export(ExportKind::Read);
    // Then
    assert_eq!(read, Err(OwnerFault::Reentry));
    assert_eq!(
        core.activation_completed(plan.ticket),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(core.epoch(), epoch);
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Activation(plan.ticket),
            TransitionPhase::Activation
        ),
        Err(OwnerFault::Reentry)
    );
    Ok(())
}
