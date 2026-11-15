use css::resident_owner::{OwnerFault, RunPlan, TransitionPhase, TransitionTicket};
use serial_test::serial;

use super::{
    bank::{Bank, Execution},
    checkpoint::HostCheckpoint,
};

#[test]
#[serial]
fn checkpoint_restores_owned_maps_when_wire_is_dropped() -> Result<(), Box<dyn std::error::Error>> {
    // Given real exported state and a parsed owned checkpoint.
    let _state = crate::TestStateGuard::new();
    crate::reset_build_state_internal();
    css::file_map::seed_file_numbers(&["/src/original.tsx".into()]);
    let parts = [
        crate::export_sheet_internal()?,
        crate::export_class_map_internal()?,
        crate::export_file_map_internal()?,
        crate::export_canonical_map_internal()?,
        crate::reservation::export_reservation_state_internal()?,
    ];
    let checkpoint = HostCheckpoint::parse(parts.each_ref().map(String::as_str))?;
    drop(parts);
    crate::reset_build_state_internal();
    // When the owned checkpoint is restored after the wire value is gone.
    checkpoint.restore();
    // Then the captured inventory, not the replacement bank, is installed.
    assert!(css::file_map::get_file_map().contains_left("/src/original.tsx"));
    Ok(())
}

#[test]
#[serial]
fn checkpoint_leaves_bank_unchanged_when_last_field_is_invalid()
-> Result<(), Box<dyn std::error::Error>> {
    // Given live allocation and otherwise valid wire fields.
    let _state = crate::TestStateGuard::new();
    let parts = [
        crate::export_sheet_internal()?,
        crate::export_class_map_internal()?,
        crate::export_file_map_internal()?,
        crate::export_canonical_map_internal()?,
    ];
    let before = crate::export_file_map_internal()?;
    // When the final checkpoint family cannot be parsed.
    let result = HostCheckpoint::parse([&parts[0], &parts[1], &parts[2], &parts[3], "invalid"]);
    // Then decoding never installs partially parsed state.
    assert!(result.is_err());
    assert_eq!(crate::export_file_map_internal()?, before);
    Ok(())
}

#[test]
fn bank_retains_first_refusal_when_callback_catches_second() -> Result<(), OwnerFault> {
    // Given an admitted configuration frame.
    let mut bank = Bank::new()?;
    let id = bank.create()?;
    let plan = bank.core.begin_run(bank.owner(id)?)?;
    let RunPlan::Activate(plan) = plan else {
        panic!("expected activation")
    };
    let ticket = bank.core.activation_completed(plan.ticket)?;
    bank.execution = Some(Execution::Configure(ticket));
    // When two caught binding refusals occur.
    assert_eq!(
        bank.latch::<()>(Err(OwnerFault::ForeignEngine)),
        Err(OwnerFault::ForeignEngine)
    );
    assert_eq!(
        bank.latch::<()>(Err(OwnerFault::Reentry)),
        Err(OwnerFault::Reentry)
    );
    // Then the first failure remains authoritative until unwind.
    assert_eq!(bank.check_fault(), Err(OwnerFault::ForeignEngine));
    assert_eq!(
        bank.core.abort_transition(
            TransitionTicket::Configuration(ticket),
            TransitionPhase::Configuration
        ),
        Err(OwnerFault::ConfigurationFailed)
    );
    assert_eq!(bank.take_fault(), Err(OwnerFault::ForeignEngine));
    Ok(())
}

#[test]
fn matching_disposal_latches_before_export_publication() -> Result<(), OwnerFault> {
    // Given an active owner's admitted extraction export.
    let mut bank = Bank::new()?;
    let id = bank.create()?;
    let RunPlan::Activate(plan) = bank.core.begin_run(bank.owner(id)?)? else {
        panic!("expected activation")
    };
    let configured = bank.core.activation_completed(plan.ticket)?;
    let running = bank.core.configuration_completed(configured)?;
    bank.execution = Some(Execution::Run(running));
    let css::resident_owner::ExportPlan::Admitted(export) = bank
        .core
        .begin_export(css::resident_owner::ExportKind::Mutate)?
    else {
        panic!("expected admitted export")
    };
    bank.export = Some((bank.issue()?, export));
    // When the resolver disposes the active owner without requesting clear.
    bank.dispose(id)?;
    // Then the prepublication binding check refuses before end_export or reset.
    assert_eq!(bank.check_fault(), Err(OwnerFault::ClosedOwner));
    assert_eq!(bank.core.end_export(export), Err(OwnerFault::ClosedOwner));
    assert_eq!(bank.core.end_run(running), Err(OwnerFault::ClosedOwner));
    Ok(())
}

#[test]
fn unrelated_disposal_leaves_active_publication_admitted() -> Result<(), OwnerFault> {
    // Given a configuring owner and an unrelated open handle.
    let mut bank = Bank::new()?;
    let active = bank.create()?;
    let unrelated = bank.create()?;
    let RunPlan::Activate(plan) = bank.core.begin_run(bank.owner(active)?)? else {
        panic!("expected activation")
    };
    let configured = bank.core.activation_completed(plan.ticket)?;
    bank.execution = Some(Execution::Configure(configured));
    // When the unrelated handle is disposed.
    bank.dispose(unrelated)?;
    // Then neither the binding latch nor the active configuration is poisoned.
    assert_eq!(bank.check_fault(), Ok(()));
    let running = bank.core.configuration_completed(configured)?;
    bank.core.end_run(running)
}
