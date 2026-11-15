use super::state::{Pending, Phase};
use super::{ClearPlan, ClearTicket, OwnerCoordinator, OwnerFault, OwnerState, OwnerToken};

impl<C> OwnerCoordinator<C> {
    /// Tombstone immediately, discarding checkpoint authority but not live tickets.
    ///
    /// # Errors
    /// Rejects foreign/unissued identities. Repeated disposal is harmless. A
    /// matching active operation is latched closed and must still unwind. This
    /// does not request a reset; call `request_clear` for targeted cleanup.
    pub fn dispose(&mut self, owner: OwnerToken) -> Result<(), OwnerFault> {
        if let Err(fault) = self.record(owner) {
            return self.refuse(fault);
        }
        let record = self
            .owners
            .get_mut(&owner)
            .ok_or(OwnerFault::InvalidOwner)?;
        record.state = OwnerState::Disposed;
        record.checkpoint = None;
        if self.resident == Some(owner) {
            self.resident = None;
        }
        if self.phase.involves(owner) || (self.export.is_some() && self.bank_owner == Some(owner)) {
            self.fault.get_or_insert(OwnerFault::ClosedOwner);
        }
        Ok(())
    }

    /// Request reset only of the bank currently attributed to this owner.
    ///
    /// # Errors
    /// Rejects foreign/unissued identities and exhausted authority. Open resident
    /// state is preserved first. Active frames defer reset, never force-unlock;
    /// deferred requests must drain with `poll_clear` before new idle work.
    pub fn request_clear(&mut self, owner: OwnerToken) -> Result<ClearPlan, OwnerFault> {
        if let Err(fault) = self.record(owner) {
            return self.refuse(fault);
        }
        let record = self
            .owners
            .get_mut(&owner)
            .ok_or(OwnerFault::InvalidOwner)?;
        record.clear = true;
        if self.export.is_some() || self.phase.key().is_some() {
            return Ok(ClearPlan::Deferred);
        }
        if self.bank_owner != Some(owner) {
            record.clear = false;
            return Ok(ClearPlan::Skipped);
        }
        match self.preserve(Pending::Clear(owner))? {
            Some(plan) => Ok(ClearPlan::Preserve(plan)),
            None => self.reset_plan(owner),
        }
    }

    /// Drain one queued owner-targeted clear after unwind; repeat until skipped.
    ///
    /// # Errors
    /// Propagates checked allocation/epoch failures. Active work returns deferred.
    /// Skips obsolete targets internally; skipped means the queue is drained.
    pub fn poll_clear(&mut self) -> Result<ClearPlan, OwnerFault> {
        loop {
            let next = self
                .owners
                .iter()
                .find(|(_, record)| record.clear)
                .map(|(owner, _)| *owner);
            match next {
                Some(owner) => match self.request_clear(owner)? {
                    ClearPlan::Skipped => {}
                    plan @ (ClearPlan::Deferred | ClearPlan::Preserve(_) | ClearPlan::Reset(_)) => {
                        return Ok(plan);
                    }
                },
                None => return Ok(ClearPlan::Skipped),
            }
        }
    }

    pub(super) fn reset_plan(&mut self, owner: OwnerToken) -> Result<ClearPlan, OwnerFault> {
        let key = self.issue_key()?;
        self.mutate()?;
        self.resident = None;
        self.phase = Phase::Clearing { key, owner };
        Ok(ClearPlan::Reset(ClearTicket(key)))
    }

    /// Complete the planned reset after all engine frames have unwound.
    ///
    /// # Errors
    /// Refuses foreign/stale tickets, active exports, and latched faults without
    /// releasing reset authority; abort a failed clear before retrying. It never
    /// clears a different owner's attribution or revives a disposed owner.
    pub fn clear_completed(&mut self, ticket: ClearTicket) -> Result<(), OwnerFault> {
        let Phase::Clearing { key, owner } = self.phase else {
            return self.reject_ticket(ticket.0);
        };
        self.check_key(ticket.0, Some(key))?;
        self.exports_unwound()?;
        self.unlatched()?;
        let record = self
            .owners
            .get_mut(&owner)
            .ok_or(OwnerFault::InvalidOwner)?;
        record.clear = false;
        self.bank_owner = None;
        self.phase = Phase::Idle;
        Ok(())
    }
}
