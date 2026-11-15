use css::resident_owner::{
    ExportKind, ExportPlan, OwnerFault, PreservationKind, ResumePlan, RunPlan, TransitionPhase,
    TransitionTicket,
};

use super::{
    bank::Execution,
    checkpoint::HostCheckpoint,
    decision,
    interpreter::{ErrorTransport, Owner},
};

pub(super) const fn kind(
    opcode: u32,
    capture: Option<css::resident_owner::CheckpointTicket>,
) -> Result<ExportKind, OwnerFault> {
    let snapshot = match opcode {
        0..=22 => return Ok(ExportKind::Mutate),
        23 => PreservationKind::Sheet,
        24 => PreservationKind::ClassMap,
        25 => PreservationKind::FileMap,
        26 => PreservationKind::CanonicalMap,
        27 => PreservationKind::ReservationState,
        28..=35 => return Ok(ExportKind::Read),
        _ => return Err(OwnerFault::WrongPhase),
    };
    Ok(match capture {
        Some(ticket) => ExportKind::Preservation(ticket, snapshot),
        None => ExportKind::Read,
    })
}

impl<E: ErrorTransport, F: Clone, T: Into<E::Value>> Owner<'_, E, F, T> {
    pub(super) fn configure(&self, plan: RunPlan<HostCheckpoint>) -> Result<(), E::Value> {
        let ticket = match plan {
            RunPlan::Preserve(plan) => match self.preserve(plan)? {
                ResumePlan::Run(next) => return self.configure(next),
                ResumePlan::Export(_) | ResumePlan::Clear(_) => {
                    return Err(self.errors.error(OwnerFault::WrongPhase));
                }
            },
            RunPlan::Activate(plan) => {
                (self.engine.reset)();
                if let Some(checkpoint) = plan.restore {
                    (self.engine.restore)(checkpoint.payload());
                }
                match self
                    .bank
                    .with(|bank| bank.core.activation_completed(plan.ticket))
                {
                    Ok(ticket) => ticket,
                    Err(fault) => {
                        return Err(self.abort(
                            TransitionTicket::Activation(plan.ticket),
                            TransitionPhase::Activation,
                            self.errors.error(fault),
                        ));
                    }
                }
            }
            RunPlan::Configure(ticket) => ticket,
        };
        self.bank
            .with(|bank| {
                bank.execution = Some(Execution::Configure(ticket));
                Ok(())
            })
            .map_err(|fault| self.errors.error(fault))
    }

    pub(super) fn begin(&self, id: u64) -> Result<(), E::Value> {
        let plan = self
            .bank
            .with(|bank| {
                let result = bank.owner(id).and_then(|owner| bank.core.begin_run(owner));
                bank.latch(result)
            })
            .map_err(|fault| self.errors.error(fault))?;
        self.configure(plan)
    }

    pub(super) fn configured(&self) -> Result<(), E::Value> {
        self.bank
            .with(|bank| {
                bank.check_fault()?;
                match bank.execution {
                    Some(Execution::Configure(ticket)) => {
                        let ticket = bank.core.configuration_completed(ticket)?;
                        bank.execution = Some(Execution::Run(ticket));
                        Ok(())
                    }
                    Some(Execution::Run(_)) | None => Err(OwnerFault::WrongPhase),
                }
            })
            .map_err(|fault| self.errors.error(fault))
    }

    pub(super) fn finish(&self) -> Result<(), E::Value> {
        let (result, latched, expected) = self
            .bank
            .with(|bank| {
                let (result, expected) = match bank.execution.take() {
                    Some(Execution::Run(ticket)) => (bank.core.end_run(ticket), None),
                    Some(Execution::Configure(ticket)) => (
                        bank.core.abort_transition(
                            TransitionTicket::Configuration(ticket),
                            TransitionPhase::Configuration,
                        ),
                        Some(OwnerFault::ConfigurationFailed),
                    ),
                    None => (Err(OwnerFault::WrongPhase), None),
                };
                Ok((result, bank.take_fault(), expected))
            })
            .map_err(|fault| self.errors.error(fault))?;
        decision::unwind(result, latched, expected).map_err(|faults| self.errors.faults(faults))
    }

    pub(super) fn admitted(&self, plan: ExportPlan, id: u64) -> Result<u64, E::Value> {
        match plan {
            ExportPlan::Preserve(plan) => match self.preserve(plan)? {
                ResumePlan::Export(next) => self.admitted(next, id),
                ResumePlan::Run(_) | ResumePlan::Clear(_) => {
                    Err(self.errors.error(OwnerFault::WrongPhase))
                }
            },
            ExportPlan::Admitted(ticket) => self
                .bank
                .with(|bank| {
                    bank.export = Some((id, ticket));
                    Ok(id)
                })
                .map_err(|fault| self.errors.error(fault)),
        }
    }

    pub(super) fn enter(&self, opcode: u32) -> Result<u64, E::Value> {
        let (plan, id) = self
            .bank
            .with(|bank| bank.enter_export(opcode))
            .map_err(|fault| self.errors.error(fault))?;
        self.admitted(plan, id)
    }

    pub(super) fn leave(&self, id: u64) -> Result<(), E::Value> {
        let (result, latched) = self
            .bank
            .with(|bank| {
                let result = match bank.export {
                    Some((actual, ticket)) if actual == id => {
                        bank.export = None;
                        bank.core.end_export(ticket)
                    }
                    Some(_) | None => Err(OwnerFault::StaleTicket),
                };
                if bank.execution.is_none() && bank.capture.is_none() {
                    Ok((result, bank.take_fault()))
                } else {
                    let result = bank.latch(result);
                    Ok((result, bank.check_fault()))
                }
            })
            .map_err(|fault| self.errors.error(fault))?;
        let outcome = decision::unwind(result, latched, None)
            .map_err(|faults| (self.context.attach_cause)(self.errors.faults(faults)));
        (self.context.clear_cause)();
        outcome
    }
}
