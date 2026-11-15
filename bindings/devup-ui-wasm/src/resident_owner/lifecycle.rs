use css::resident_owner::{
    ClearPlan, OwnerFault, OwnerState, ResumePlan, TransitionPhase, TransitionTicket,
};

use super::interpreter::{ErrorTransport, Owner};

impl<E: ErrorTransport, F: Clone, T: Into<E::Value>> Owner<'_, E, F, T> {
    pub(super) fn execute(&self, plan: ClearPlan) -> Result<bool, E::Value> {
        match plan {
            ClearPlan::Preserve(plan) => match self.preserve(plan)? {
                ResumePlan::Clear(next) => self.execute(next),
                ResumePlan::Run(_) | ResumePlan::Export(_) => {
                    Err(self.errors.error(OwnerFault::WrongPhase))
                }
            },
            ClearPlan::Reset(ticket) => {
                (self.engine.reset)();
                match self.bank.with(|bank| bank.core.clear_completed(ticket)) {
                    Ok(()) => Ok(true),
                    Err(fault) => Err(self.abort(
                        TransitionTicket::Clear(ticket),
                        TransitionPhase::Clear,
                        self.errors.error(fault),
                    )),
                }
            }
            ClearPlan::Deferred | ClearPlan::Skipped => Ok(false),
        }
    }

    pub(super) fn dispose(&self, id: u64) -> Result<(), E::Value> {
        self.bank
            .with(|bank| bank.dispose(id))
            .map_err(|fault| self.errors.error(fault))?;
        self.callbacks
            .with_borrow_mut(|callbacks| callbacks.remove(&id));
        if self.bank.check_fault().is_err() {
            (self.context.retain_importer)();
        }
        Ok(())
    }

    pub(super) fn clear(&self, id: u64) -> Result<(), E::Value> {
        let plan = self
            .bank
            .with(|bank| {
                let owner = bank.owner(id)?;
                bank.core.request_clear(owner)
            })
            .map_err(|fault| self.errors.error(fault))?;
        let outcome = self.execute(plan).map(|_| ());
        self.errors.complete(outcome, self.drain())
    }

    pub(super) fn drain(&self) -> Result<(), E::Value> {
        loop {
            let plan = self
                .bank
                .with(|bank| bank.core.poll_clear())
                .map_err(|fault| self.errors.error(fault))?;
            if !self.execute(plan)? {
                return Ok(());
            }
        }
    }

    pub(super) fn disposed(&self, id: u64) -> Result<bool, E::Value> {
        self.bank
            .with(|bank| {
                let owner = bank.owner(id)?;
                bank.core.owner_state(owner).map(|state| match state {
                    OwnerState::Open => false,
                    OwnerState::Disposed | OwnerState::Faulted(_) => true,
                })
            })
            .map_err(|fault| self.errors.error(fault))
    }
}
