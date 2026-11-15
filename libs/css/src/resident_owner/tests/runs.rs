use super::*;

#[test]
fn activation_is_only_a_plan_until_configuration_completes() -> TestResult {
    // Given
    let mut core = Core::new()?;
    let owner = core.create_owner()?;
    // When
    let RunPlan::Activate(plan) = core.begin_run(owner)? else {
        panic!("activation required")
    };
    // Then
    assert!(plan.restore.is_none());
    assert_eq!(core.bank_owner(), Some(owner));
    assert_eq!(core.resident_owner(), None);
    assert_eq!(core.epoch(), Epoch(1));
    let configuration = core.activation_completed(plan.ticket)?;
    assert_eq!(core.resident_owner(), None);
    let run = core.configuration_completed(configuration)?;
    assert_eq!(core.resident_owner(), Some(owner));
    core.end_run(run)
}

#[test]
fn same_resident_replays_configuration_without_reset_restore_or_capture() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let epoch = core.epoch();
    // When
    let plan = core.begin_run(owner)?;
    // Then
    let RunPlan::Configure(configuration) = plan else {
        panic!("resident fast path")
    };
    assert_eq!(core.epoch(), epoch);
    let run = core.configuration_completed(configuration)?;
    core.end_run(run)?;
    assert!(core.checkpoint(owner)?.is_none());
    Ok(())
}

#[test]
fn takeover_preserves_before_any_destructive_activation_admission() -> TestResult {
    // Given
    let (mut core, outgoing) = resident()?;
    let incoming = core.create_owner()?;
    let epoch = core.epoch();
    // When
    let plan = takeover(&mut core, incoming)?;
    // Then
    assert_eq!(plan.stamp.engine(), core.engine_id());
    assert_eq!(plan.stamp.owner(), outgoing);
    assert_eq!(plan.stamp.epoch(), epoch);
    assert_eq!(core.epoch(), epoch);
    assert_eq!(core.bank_owner(), Some(outgoing));
    assert_eq!(core.resident_owner(), Some(outgoing));
    assert!(core.checkpoint(outgoing)?.is_none());
    Ok(())
}

#[test]
fn completed_checkpoint_survives_takeover_and_is_returned_as_owned_restore() -> TestResult {
    // Given
    let (mut core, outgoing) = resident()?;
    let incoming = core.create_owner()?;
    let preservation = takeover(&mut core, incoming)?;
    let activation = resume_run(core.checkpoint_completed(preservation.ticket, vec![2, 7])?);
    let configuration = core.activation_completed(activation.ticket)?;
    let run = core.configuration_completed(configuration)?;
    core.end_run(run)?;
    let second = takeover(&mut core, outgoing)?;
    // When
    let restore = resume_run(core.checkpoint_completed(second.ticket, vec![9])?);
    // Then
    let checkpoint = restore.restore.ok_or(OwnerFault::CaptureFailed)?;
    assert_eq!(checkpoint.payload(), &[2, 7]);
    assert_eq!(checkpoint.stamp(), preservation.stamp);
    assert_eq!(core.epoch(), Epoch(3));
    assert_eq!(
        core.checkpoint(incoming)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[9]
    );
    Ok(())
}

#[test]
fn action_throw_retains_partial_allocations_without_return_time_capture() -> TestResult {
    // Given
    let (mut core, owner) = resident()?;
    let run = start(&mut core, owner)?;
    let mut simulated_bank = vec![1];
    let export = admitted(core.begin_export(ExportKind::Mutate)?);
    simulated_bank.push(4);
    core.end_export(export)?;
    // When: the action throws after an allocation; the host unwinds its run ticket.
    core.end_run(run)?;
    // Then
    assert_eq!(simulated_bank, [1, 4]);
    assert_eq!(core.resident_owner(), Some(owner));
    assert!(core.checkpoint(owner)?.is_none());
    let other = core.create_owner()?;
    let preservation = takeover(&mut core, other)?;
    core.checkpoint_completed(preservation.ticket, simulated_bank)?;
    assert_eq!(
        core.checkpoint(owner)?
            .ok_or(OwnerFault::CaptureFailed)?
            .payload(),
        &[1, 4]
    );
    Ok(())
}

#[test]
fn independent_banks_can_run_while_one_bank_refuses_both_owner_reentries() -> TestResult {
    // Given
    let mut left = Core::new()?;
    let mut right = Core::new()?;
    let owner = left.create_owner()?;
    let other = left.create_owner()?;
    let right_owner = right.create_owner()?;
    let run = start(&mut left, owner)?;
    let before = left.epoch();
    // When
    let same = left.begin_run(owner).err();
    let different = left.begin_run(other).err();
    let independent = start(&mut right, right_owner)?;
    // Then
    assert_eq!(
        (same, different),
        (Some(OwnerFault::Reentry), Some(OwnerFault::Reentry))
    );
    assert_eq!(left.epoch(), before);
    assert_eq!(left.end_run(run), Err(OwnerFault::Reentry));
    assert_eq!(left.resident_owner(), Some(owner));
    right.end_run(independent)
}
