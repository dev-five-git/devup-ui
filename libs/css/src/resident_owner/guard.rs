use super::identity::{Key, advance};
use super::state::{OwnerRecord, Pending, Phase};
use super::{
    CheckpointStamp, CheckpointTicket, Counter, OwnerCoordinator, OwnerFault, OwnerToken,
    PreservationPlan,
};

impl<C> OwnerCoordinator<C> {
    pub(super) fn record(&self, owner: OwnerToken) -> Result<&OwnerRecord<C>, OwnerFault> {
        if owner.engine != self.engine {
            return Err(OwnerFault::ForeignEngine);
        }
        self.owners.get(&owner).ok_or(OwnerFault::InvalidOwner)
    }

    pub(super) fn refuse<T>(&mut self, fault: OwnerFault) -> Result<T, OwnerFault> {
        if self.phase.key().is_some() || self.export.is_some() {
            self.fault.get_or_insert(fault);
        }
        Err(fault)
    }

    pub(super) fn require_open(&mut self, owner: OwnerToken) -> Result<(), OwnerFault> {
        match self.record(owner).and_then(OwnerRecord::open) {
            Ok(()) => Ok(()),
            Err(fault) => self.refuse(fault),
        }
    }

    pub(super) fn check_key(&mut self, key: Key, expected: Option<Key>) -> Result<(), OwnerFault> {
        if key.engine != self.engine || expected != Some(key) {
            return self.reject_ticket(key);
        }
        Ok(())
    }

    pub(super) fn reject_ticket<T>(&mut self, key: Key) -> Result<T, OwnerFault> {
        let fault = if key.engine == self.engine {
            OwnerFault::StaleTicket
        } else {
            OwnerFault::ForeignEngine
        };
        self.refuse(fault)
    }

    pub(super) const fn unlatched(&self) -> Result<(), OwnerFault> {
        match self.fault {
            Some(fault) => Err(fault),
            None => Ok(()),
        }
    }

    pub(super) fn exports_unwound(&mut self) -> Result<(), OwnerFault> {
        if self.export.is_some() {
            return self.refuse(OwnerFault::Reentry);
        }
        Ok(())
    }

    pub(super) fn idle(&mut self) -> Result<(), OwnerFault> {
        self.exports_unwound()?;
        if self.phase.key().is_none() {
            Ok(())
        } else {
            self.refuse(OwnerFault::Reentry)
        }
    }

    pub(super) fn cleanup_drained(&self) -> Result<(), OwnerFault> {
        if self.owners.values().any(|record| record.clear) {
            return Err(OwnerFault::CleanupPending);
        }
        Ok(())
    }

    pub(super) fn issue_key(&mut self) -> Result<Key, OwnerFault> {
        match advance(&mut self.ticket_sequence, Counter::Ticket) {
            Ok(sequence) => Ok(Key {
                engine: self.engine,
                sequence,
            }),
            Err(fault) => self.refuse(fault),
        }
    }

    pub(super) fn mutate(&mut self) -> Result<(), OwnerFault> {
        match advance(&mut self.epoch.0, Counter::Epoch) {
            Ok(_) => Ok(()),
            Err(fault) => self.refuse(fault),
        }
    }

    pub(super) fn preserve(
        &mut self,
        next: Pending,
    ) -> Result<Option<PreservationPlan>, OwnerFault> {
        match self.resident {
            Some(owner) => {
                self.require_open(owner)?;
                let key = self.issue_key()?;
                let stamp = CheckpointStamp {
                    owner,
                    epoch: self.epoch,
                };
                self.phase = Phase::Preserving { key, stamp, next };
                Ok(Some(PreservationPlan {
                    ticket: CheckpointTicket(key),
                    stamp,
                }))
            }
            None => Ok(None),
        }
    }
}
