use super::*;

#[test]
fn unmanaged_mutation_waits_for_capture_then_revokes_old_attribution() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let epoch = core.epoch();
    // When
    let ExportPlan::Preserve(plan) = core.begin_export(ExportKind::Mutate)? else {
        panic!("capture first")
    };
    // Then
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.bank_owner(), Some(owner));
    let ResumePlan::Export(ExportPlan::Admitted(ticket)) =
        core.checkpoint_completed(plan.ticket, vec![8])?
    else {
        panic!("mutation admission")
    };
    assert_eq!(core.epoch(), Epoch(2));
    assert_eq!(core.bank_owner(), None);
    assert_eq!(core.resident_owner(), None);
    assert_eq!(
        core.checkpoint(owner)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[8]
    );
    core.end_export(ticket)?;
    assert_eq!(core.request_clear(owner)?, ClearPlan::Skipped);
    Ok(())
}

#[test]
fn mutation_epoch_advances_before_marshalling_even_when_host_conversion_throws() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let run = start(&mut core, owner)?;
    let before = core.epoch();
    // When
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    // Then
    assert_ne!(core.epoch(), before);
    assert_eq!(core.epoch(), Epoch(2));
    core.end_export(export)?;
    core.end_run(run)?;
    assert_eq!(core.resident_owner(), Some(owner));
    Ok(())
}

#[test]
fn reads_guard_the_bank_without_charging_epoch_or_revoking_residency() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let epoch = core.epoch();
    // When
    let export = admitted(core.begin_export(ExportKind::Read)?);
    // Then
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.resident_owner(), Some(owner));
    assert_eq!(
        core.begin_export(ExportKind::Read),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(core.end_export(export), Err(OwnerFault::Reentry));
    let next = admitted(core.begin_export(ExportKind::Read)?);
    core.end_export(next)
}

#[test]
fn capture_admits_only_sequential_matching_allowlisted_exports() -> TestResult {
    // Given
    let (mut core, _) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let epoch = core.epoch();
    // When
    for kind in [
        PreservationKind::Sheet,
        PreservationKind::ClassMap,
        PreservationKind::FileMap,
        PreservationKind::CanonicalMap,
        PreservationKind::ReservationState,
    ] {
        let export = admitted(core.begin_export(ExportKind::Preservation(capture.ticket, kind))?);
        core.end_export(export)?;
    }
    // Then
    assert_eq!(core.epoch(), epoch);
    let activation = resume_run(core.checkpoint_completed(capture.ticket, vec![1, 2, 3])?);
    assert!(activation.restore.is_none());
    Ok(())
}

#[test]
fn caught_nested_read_during_capture_cannot_commit_a_false_checkpoint() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    let export = admitted(core.begin_export(ExportKind::Preservation(
        capture.ticket,
        PreservationKind::Sheet,
    ))?);
    // When
    let refusal = core.begin_export(ExportKind::Read);
    // Then
    assert_eq!(refusal, Err(OwnerFault::Reentry));
    assert_eq!(core.end_export(export), Err(OwnerFault::Reentry));
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![99]).err(),
        Some(OwnerFault::Reentry)
    );
    assert!(core.checkpoint(owner)?.is_none());
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Preservation(capture.ticket),
            TransitionPhase::Preservation
        ),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(
        core.owner_state(owner)?,
        OwnerState::Faulted(OwnerFault::CaptureFailed)
    );
    Ok(())
}

#[test]
fn caught_nested_mutation_remains_latched_until_action_unwind() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let run = start(&mut core, owner)?;
    let export = admitted(core.begin_export(ExportKind::Read)?);
    let epoch = core.epoch();
    // When
    let refusal = core.begin_export(ExportKind::Mutate);
    // Then
    assert_eq!(refusal, Err(OwnerFault::Reentry));
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.end_export(export), Err(OwnerFault::Reentry));
    assert_eq!(
        core.begin_export(ExportKind::Read),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(core.end_run(run), Err(OwnerFault::Reentry));
    let next = start(&mut core, owner)?;
    core.end_run(next)
}

#[test]
fn configuration_exports_are_admitted_but_action_cannot_overtake_a_live_export() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let RunPlan::Configure(configuration) = core.begin_run(owner)? else {
        panic!("configuration")
    };
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    // When
    let completion = core.configuration_completed(configuration);
    // Then
    assert_eq!(completion, Err(OwnerFault::Reentry));
    assert_eq!(core.end_export(export), Err(OwnerFault::Reentry));
    assert_eq!(
        core.configuration_completed(configuration),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(
        core.abort_transition(
            TransitionTicket::Configuration(configuration),
            TransitionPhase::Configuration
        ),
        Err(OwnerFault::Reentry)
    );
    assert_eq!(core.resident_owner(), None);
    Ok(())
}

#[test]
fn unmanaged_initial_mutation_and_managed_reads_are_sequentially_admitted() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    core.end_export(export)?;
    let owner = core.create_owner()?;
    let run = start(&mut core, owner)?;
    let before = core.epoch();
    // When
    let read = admitted(core.begin_export(ExportKind::Read)?);
    core.end_export(read)?;
    // Then
    assert_eq!(core.epoch(), before);
    core.end_run(run)
}
