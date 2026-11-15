use super::state::{Pending, Phase};
use super::{
    ActivationPlan, ActivationTicket, ConfigurationTicket, OwnerCoordinator, OwnerFault,
    OwnerToken, RunPlan, RunTicket,
};

impl<C> OwnerCoordinator<C> {
    /// Begin synchronous work, preserving an outgoing resident before takeover.
    ///
    /// # Errors
    /// Refuses reentry before mutation, invalid/closed/faulted owners, pending
    /// cleanup, and exhausted identities/epochs. Same-owner residency skips capture
    /// and reset/restore, but never skips configuration replay.
    pub fn begin_run(&mut self, owner: OwnerToken) -> Result<RunPlan<C>, OwnerFault> {
        self.idle()?;
        self.cleanup_drained()?;
        self.require_open(owner)?;
        if self.resident == Some(owner) {
            let key = self.issue_key()?;
            self.resident = None;
            self.phase = Phase::Configuring { key, owner };
            return Ok(RunPlan::Configure(ConfigurationTicket(key)));
        }
        match self.preserve(Pending::Run(owner))? {
            Some(plan) => Ok(RunPlan::Preserve(plan)),
            None => self.activate(owner).map(RunPlan::Activate),
        }
    }

    pub(super) fn activate(&mut self, owner: OwnerToken) -> Result<ActivationPlan<C>, OwnerFault> {
        self.require_open(owner)?;
        let restore = self.checkpoint(owner)?;
        let key = self.issue_key()?;
        self.mutate()?;
        self.resident = None;
        self.bank_owner = Some(owner);
        self.phase = Phase::Activating { key, owner };
        Ok(ActivationPlan {
            ticket: ActivationTicket(key),
            restore,
        })
    }

    /// Admit action only after all configuration replay has completed.
    ///
    /// # Errors
    /// Refuses stale/foreign tickets, live exports, latched faults, closed owners,
    /// and exhausted ticket identities. On refusal, abort configuration to unwind.
    pub fn configuration_completed(
        &mut self,
        ticket: ConfigurationTicket,
    ) -> Result<RunTicket, OwnerFault> {
        let Phase::Configuring { key, owner } = self.phase else {
            return self.reject_ticket(ticket.0);
        };
        self.check_key(ticket.0, Some(key))?;
        self.exports_unwound()?;
        self.unlatched()?;
        self.require_open(owner)?;
        let key = self.issue_key()?;
        self.phase = Phase::Running { key, owner };
        self.resident = Some(owner);
        Ok(RunTicket(key))
    }

    /// End action on return or throw, retaining actual live/partial effects.
    ///
    /// # Errors
    /// Stale/foreign tickets and live exports cannot unwind this run. A valid
    /// ticket unwinds even with a latched refusal/closed owner and returns that
    /// fault, so catching a nested refusal cannot publish success. No capture or
    /// rollback occurs; the host retains its original action error separately.
    pub fn end_run(&mut self, ticket: RunTicket) -> Result<(), OwnerFault> {
        let Phase::Running { key, owner } = self.phase else {
            return self.reject_ticket(ticket.0);
        };
        self.check_key(ticket.0, Some(key))?;
        self.exports_unwound()?;
        let open = self.record(owner)?.open();
        self.resident = open.is_ok().then_some(owner);
        self.phase = Phase::Idle;
        match self.fault.take() {
            Some(fault) => Err(fault),
            None => open,
        }
    }
}
