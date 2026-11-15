use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn engine_id_exhaustion_refuses_without_wrapping_or_recycling() {
    // Given
    let source = AtomicUsize::new(usize::MAX - 1);
    // When
    let final_id = EngineId::allocate(&source);
    let overflow = EngineId::allocate(&source);
    // Then
    assert_eq!(final_id, Ok(EngineId(usize::MAX - 1)));
    assert_eq!(overflow, Err(OwnerFault::Exhausted(Counter::Engine)));
    assert_eq!(source.load(Ordering::Relaxed), usize::MAX);
}

#[test]
fn owner_id_exhaustion_never_reuses_a_disposed_identity() -> TestResult {
    // Given
    let mut core = Core::new()?;
    core.owner_sequence = u64::MAX - 1;
    let last = core.create_owner()?;
    core.dispose(last)?;
    // When
    let overflow = core.create_owner();
    // Then
    assert_eq!(overflow, Err(OwnerFault::Exhausted(Counter::Owner)));
    assert_eq!(core.owner_state(last)?, OwnerState::Disposed);
    assert_eq!(core.owner_sequence, u64::MAX);
    Ok(())
}

#[test]
fn ticket_exhaustion_prevents_activation_without_epoch_or_bank_changes() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let owner = core.create_owner()?;
    core.ticket_sequence = u64::MAX;
    // When
    let overflow = core.begin_run(owner).err();
    // Then
    assert_eq!(overflow, Some(OwnerFault::Exhausted(Counter::Ticket)));
    assert_eq!(core.epoch(), Epoch(0));
    assert_eq!(core.bank_owner(), None);
    Ok(())
}

#[test]
fn epoch_exhaustion_refuses_before_unmanaged_mutation_admission() -> TestResult {
    // Given
    let mut core = Core::new()?;
    core.epoch = Epoch(u64::MAX);
    // When
    let overflow = core.begin_export(ExportKind::Mutate);
    // Then
    assert_eq!(overflow, Err(OwnerFault::Exhausted(Counter::Epoch)));
    assert_eq!(core.epoch(), Epoch(u64::MAX));
    let read = admitted(core.begin_export(ExportKind::Read)?);
    core.end_export(read)
}

#[test]
fn completed_outgoing_capture_survives_next_activation_epoch_exhaustion() -> TestResult {
    // Given
    let (mut core, outgoing) = resident()?;
    let incoming = core.create_owner()?;
    core.epoch = Epoch(u64::MAX);
    let capture = takeover(&mut core, incoming)?;
    // When
    let overflow = core.checkpoint_completed(capture.ticket, vec![41]).err();
    // Then
    assert_eq!(overflow, Some(OwnerFault::Exhausted(Counter::Epoch)));
    assert_eq!(
        core.checkpoint(outgoing)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[41]
    );
    assert_eq!(core.bank_owner(), Some(outgoing));
    assert_eq!(core.resident_owner(), Some(outgoing));
    assert_eq!(
        core.checkpoint_completed(capture.ticket, vec![0]).err(),
        Some(OwnerFault::StaleTicket)
    );
    Ok(())
}

#[test]
fn managed_epoch_overflow_is_latched_without_losing_live_partial_state() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let run = start(&mut core, owner)?;
    core.epoch = Epoch(u64::MAX);
    // When
    let overflow = core.begin_export(ExportKind::Mutate);
    // Then
    assert_eq!(overflow, Err(OwnerFault::Exhausted(Counter::Epoch)));
    assert_eq!(
        core.end_run(run),
        Err(OwnerFault::Exhausted(Counter::Epoch))
    );
    assert_eq!(core.resident_owner(), Some(owner));
    Ok(())
}
