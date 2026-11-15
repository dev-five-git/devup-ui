use super::*;

struct OldTickets {
    activation: ActivationTicket,
    configuration: ConfigurationTicket,
    run: RunTicket,
    capture: CheckpointTicket,
    clear: ClearTicket,
}

fn closed_cycle(core: &mut Core) -> Result<OldTickets, OwnerFault> {
    let owner = core.create_owner()?;
    let other = core.create_owner()?;
    let RunPlan::Activate(plan) = core.begin_run(owner)? else {
        panic!("activation")
    };
    let configuration = core.activation_completed(plan.ticket)?;
    let run = core.configuration_completed(configuration)?;
    core.end_run(run)?;
    let capture = takeover(core, other)?;
    let next = resume_run(core.checkpoint_completed(capture.ticket, vec![8])?);
    let next_configuration = core.activation_completed(next.ticket)?;
    let next_run = core.configuration_completed(next_configuration)?;
    core.end_run(next_run)?;
    core.dispose(other)?;
    let ClearPlan::Reset(clear) = core.request_clear(other)? else {
        panic!("reset")
    };
    core.clear_completed(clear)?;
    Ok(OldTickets {
        activation: plan.ticket,
        configuration,
        run,
        capture: capture.ticket,
        clear,
    })
}

#[test]
fn closed_phase_tickets_cannot_complete_or_abort_after_their_cycle_unwinds() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let old = closed_cycle(&mut core)?;
    // When
    let outcomes = [
        core.activation_completed(old.activation).err(),
        core.configuration_completed(old.configuration).err(),
        core.end_run(old.run).err(),
        core.checkpoint_completed(old.capture, vec![]).err(),
        core.clear_completed(old.clear).err(),
        core.abort_transition(
            TransitionTicket::Configuration(old.configuration),
            TransitionPhase::Configuration,
        )
        .err(),
    ];
    // Then
    assert_eq!(outcomes, [Some(OwnerFault::StaleTicket); 6]);
    assert_eq!(core.bank_owner(), None);
    Ok(())
}

#[test]
fn foreign_phase_tickets_cannot_complete_or_abort_a_different_bank() -> TestResult {
    // Given
    let mut source = Core::new()?;
    let foreign = closed_cycle(&mut source)?;
    let mut target = Core::new()?;
    // When
    let outcomes = [
        target.activation_completed(foreign.activation).err(),
        target.configuration_completed(foreign.configuration).err(),
        target.end_run(foreign.run).err(),
        target.checkpoint_completed(foreign.capture, vec![]).err(),
        target.clear_completed(foreign.clear).err(),
        target
            .abort_transition(
                TransitionTicket::Clear(foreign.clear),
                TransitionPhase::Clear,
            )
            .err(),
    ];
    // Then
    assert_eq!(outcomes, [Some(OwnerFault::ForeignEngine); 6]);
    assert_eq!(target.epoch(), Epoch(0));
    Ok(())
}

#[test]
fn closed_export_ticket_is_stale_after_return_processing_unwinds() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let export = admitted(core.begin_export(ExportKind::Read)?);
    core.end_export(export)?;
    // When
    let refusal = core.end_export(export);
    // Then
    assert_eq!(refusal, Err(OwnerFault::StaleTicket));
    let next = admitted(core.begin_export(ExportKind::Mutate)?);
    core.end_export(next)
}

#[test]
fn disposing_an_unrelated_owner_during_unmanaged_capture_does_not_poison_capture() -> TestResult {
    // Given
    let (mut core, _) = resident()?;
    let unrelated = core.create_owner()?;
    let ExportPlan::Preserve(capture) = core.begin_export(ExportKind::Mutate)? else {
        panic!("capture")
    };
    // When
    core.dispose(unrelated)?;
    // Then
    let ResumePlan::Export(ExportPlan::Admitted(export)) =
        core.checkpoint_completed(capture.ticket, vec![1])?
    else {
        panic!("admitted mutation")
    };
    core.end_export(export)
}

#[test]
fn arbitrary_read_during_capture_is_not_a_preservation_export() -> TestResult {
    // Given
    let (mut core, _) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    // When
    let refusal = core.begin_export(ExportKind::Read);
    // Then
    assert_eq!(refusal, Err(OwnerFault::Reentry));
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
fn ticket_exhaustion_after_capture_keeps_a_once_committed_checkpoint() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let incoming = core.create_owner()?;
    let capture = takeover(&mut core, incoming)?;
    core.ticket_sequence = u64::MAX;
    // When
    let refusal = core.checkpoint_completed(capture.ticket, vec![71]).err();
    // Then
    assert_eq!(refusal, Some(OwnerFault::Exhausted(Counter::Ticket)));
    assert_eq!(
        core.checkpoint(owner)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[71]
    );
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![99]).err(),
        Some(OwnerFault::StaleTicket)
    );
    Ok(())
}
