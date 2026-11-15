use std::sync::Arc;

use super::state::{Pending, Phase};
use super::{
    ActivationTicket, Checkpoint, CheckpointTicket, ConfigurationTicket, ExportPlan,
    OwnerCoordinator, OwnerFault, ResumePlan, RunPlan,
};

impl<C> OwnerCoordinator<C> {
    /// Commit one whole capture, then authorize the original requested next step.
    ///
    /// # Errors
    /// Rejects foreign/stale tickets, live exports, closed owners, and latched
    /// refusals without committing. A completed checkpoint is retained even if
    /// allocating the following activation/export/clear plan fails. No engine
    /// mutation is performed by this method.
    pub fn checkpoint_completed(
        &mut self,
        ticket: CheckpointTicket,
        payload: C,
    ) -> Result<ResumePlan<C>, OwnerFault> {
        let Phase::Preserving { key, stamp, next } = self.phase else {
            return self.reject_ticket(ticket.0);
        };
        self.check_key(ticket.0, Some(key))?;
        self.exports_unwound()?;
        self.unlatched()?;
        self.require_open(stamp.owner)?;
        let record = self
            .owners
            .get_mut(&stamp.owner)
            .ok_or(OwnerFault::InvalidOwner)?;
        record.checkpoint = Some(Arc::new(Checkpoint { stamp, payload }));
        self.phase = Phase::Idle;
        match next {
            Pending::Run(owner) => self
                .activate(owner)
                .map(|plan| ResumePlan::Run(RunPlan::Activate(plan))),
            Pending::Mutate => self
                .admit_mutation()
                .map(|ticket| ResumePlan::Export(ExportPlan::Admitted(ticket))),
            Pending::Clear(owner) => self.reset_plan(owner).map(ResumePlan::Clear),
        }
    }

    /// Complete exactly the planned reset/restore; configuration is still required.
    ///
    /// # Errors
    /// Rejects foreign/stale tickets, live exports, closed owners, latched faults,
    /// and exhausted ticket identities. Abort activation after a failed completion.
    pub fn activation_completed(
        &mut self,
        ticket: ActivationTicket,
    ) -> Result<ConfigurationTicket, OwnerFault> {
        let Phase::Activating { key, owner } = self.phase else {
            return self.reject_ticket(ticket.0);
        };
        self.check_key(ticket.0, Some(key))?;
        self.exports_unwound()?;
        self.unlatched()?;
        self.require_open(owner)?;
        let key = self.issue_key()?;
        self.phase = Phase::Configuring { key, owner };
        Ok(ConfigurationTicket(key))
    }
}
