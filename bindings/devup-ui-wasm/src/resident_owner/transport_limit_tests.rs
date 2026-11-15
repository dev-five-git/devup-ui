use css::resident_owner::{Counter, ExportPlan, OwnerFault, RunPlan};

use super::{Bank, Execution};
use crate::resident_owner::decision;

#[test]
#[serial_test::serial]
fn interpreter_max_refusal_latches_before_publication_without_releasing_tickets()
-> Result<(), OwnerFault> {
    use crate::resident_owner::owner_native_tests::*;
    // Given a real interpreted run and outer mutation export.
    let _fixture = Fixture::new()?;
    let id = create();
    ok(OWNER.begin(id));
    ok(OWNER.configured());
    let export = ok(OWNER.enter(0));
    ACCESS.with(|bank| {
        bank.sequence = u64::MAX;
        Ok(())
    })?;
    // When a nested scalar admission exhausts the actual transport counter.
    let exhausted = OwnerFault::Exhausted(css::resident_owner::Counter::Ticket);
    fault(OWNER.enter(0), exhausted);
    // Then publication, exact leave and run finish all retain the first refusal.
    assert_eq!(ACCESS.check_fault(), Err(exhausted));
    fault(OWNER.leave(export), exhausted);
    fault(OWNER.finish(), exhausted);
    assert_eq!(ACCESS.check_fault(), Ok(()));
    Ok(())
}

#[test]
fn transport_max_latches_before_publication_when_nested_export_is_caught() -> Result<(), OwnerFault>
{
    // Given a real owner/run and an outer export admitted by the production seam.
    let mut bank = Bank::new()?;
    let owner = bank.create()?;
    let RunPlan::Activate(plan) = bank.core.begin_run(bank.owner(owner)?)? else {
        panic!("expected activation")
    };
    let configured = bank.core.activation_completed(plan.ticket)?;
    let running = bank.core.configuration_completed(configured)?;
    bank.execution = Some(Execution::Run(running));
    let (ExportPlan::Admitted(outer), outer_id) = bank.enter_export(0)? else {
        panic!("expected outer export")
    };
    bank.export = Some((outer_id, outer));
    let epoch = bank.core.epoch();
    bank.sequence = u64::MAX;
    let exhausted = OwnerFault::Exhausted(Counter::Ticket);

    // When a callback catches a nested public-export admission at actual transport MAX.
    assert!(matches!(bank.enter_export(0), Err(fault) if fault == exhausted));

    // Then no inner ticket/epoch is admitted, and the publication check retains exhaustion.
    assert_eq!(bank.sequence, u64::MAX);
    assert_eq!(bank.export, Some((outer_id, outer)));
    assert_eq!(bank.core.epoch(), epoch);
    assert_eq!(bank.check_fault(), Err(exhausted));
    // A later refusal cannot replace the first fault or release the enclosing run.
    assert_eq!(
        bank.latch::<()>(Err(OwnerFault::Reentry)),
        Err(OwnerFault::Reentry)
    );
    bank.export = None;
    let export_result = decision::unwind(bank.core.end_export(outer), bank.check_fault(), None);
    assert_eq!(
        export_result,
        Err(decision::Faults {
            primary: exhausted,
            cleanup: None
        })
    );
    assert!(matches!(bank.execution, Some(Execution::Run(ticket)) if ticket == running));
    assert_eq!(bank.check_fault(), Err(exhausted));
    bank.execution = None;
    let completion = bank.core.end_run(running);
    assert_eq!(
        decision::unwind(completion, bank.take_fault(), None),
        export_result
    );
    Ok(())
}

#[test]
fn transport_max_leaves_idle_bank_unlatched() -> Result<(), OwnerFault> {
    // Given an idle bank with its actual private transport sequence exhausted.
    let mut bank = Bank::new()?;
    bank.sequence = u64::MAX;
    let epoch = bank.core.epoch();
    // When scalar production admission refuses a mutation.
    assert!(matches!(
        bank.enter_export(0),
        Err(OwnerFault::Exhausted(Counter::Ticket))
    ));
    // Then it neither invents an active fault nor admits any mutation or ticket.
    assert_eq!(bank.sequence, u64::MAX);
    assert_eq!(bank.core.epoch(), epoch);
    assert_eq!(bank.export, None);
    assert_eq!(bank.check_fault(), Ok(()));
    Ok(())
}
