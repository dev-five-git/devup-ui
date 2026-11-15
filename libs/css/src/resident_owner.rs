//! Engine-agnostic ownership protocol for one physical state bank.
//!
//! Construct exactly one coordinator per bank, not per adapter/root/thread. The
//! host executes owned plans without retaining a coordinator borrow, then reports
//! completion with the matching ticket. This module never reads/writes the engine
//! and never calls the host. Dropping a live ticket does not release admission:
//! the host must complete or abort it even when callbacks throw. All execution is
//! synchronous; a returned promise must not retain a ticket across an await.

use std::collections::BTreeMap;
use std::sync::Arc;

mod abort;
mod clear;
mod export;
mod fault;
mod guard;
mod identity;
mod plan;
mod run;
mod state;
mod transition;

pub use fault::{Counter, OwnerFault, TransitionPhase};
pub use identity::{
    ActivationTicket, CheckpointStamp, CheckpointTicket, ClearTicket, ConfigurationTicket,
    EngineId, Epoch, ExportTicket, OwnerToken, RunTicket,
};
pub use plan::{
    ActivationPlan, Checkpoint, ClearPlan, ExportKind, ExportPlan, PreservationKind,
    PreservationPlan, ResumePlan, RunPlan, TransitionTicket,
};
use state::{OwnerRecord, Phase};

/// Metadata-only owner lifecycle; a faulted owner cannot begin another run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerState {
    /// Eligible for admission.
    Open,
    /// Capture or activation/configuration failed; replace this owner.
    Faulted(OwnerFault),
    /// Permanently tombstoned, even if tickets are still unwinding.
    Disposed,
}

/// Single authority for a bank. `C` is a complete immutable checkpoint value.
pub struct OwnerCoordinator<C> {
    engine: EngineId,
    owners: BTreeMap<OwnerToken, OwnerRecord<C>>,
    owner_sequence: u64,
    ticket_sequence: u64,
    epoch: Epoch,
    bank_owner: Option<OwnerToken>,
    resident: Option<OwnerToken>,
    phase: Phase,
    export: Option<ExportTicket>,
    fault: Option<OwnerFault>,
}

impl<C> OwnerCoordinator<C> {
    /// Create authority for a previously uncoordinated physical bank.
    ///
    /// # Errors
    /// Returns `Exhausted(Engine)` rather than recycling an engine identity.
    pub fn new() -> Result<Self, OwnerFault> {
        let engine = EngineId::fresh()?;
        Ok(Self {
            engine,
            owners: BTreeMap::new(),
            owner_sequence: 0,
            ticket_sequence: 0,
            epoch: Epoch::default(),
            bank_owner: None,
            resident: None,
            phase: Phase::Idle,
            export: None,
            fault: None,
        })
    }

    /// Issue a new open owner without touching the physical bank.
    ///
    /// # Errors
    /// Returns `Exhausted(Owner)` without recycling an identity.
    pub fn create_owner(&mut self) -> Result<OwnerToken, OwnerFault> {
        let sequence = identity::advance(&mut self.owner_sequence, Counter::Owner)?;
        let owner = OwnerToken {
            engine: self.engine,
            sequence,
        };
        self.owners.insert(owner, OwnerRecord::new());
        Ok(owner)
    }

    /// Identity of this bank's authority, never an adapter or root identity.
    pub const fn engine_id(&self) -> EngineId {
        self.engine
    }

    /// Last admitted physical mutation epoch, including incomplete writes.
    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Physical attribution survives invalid activation and tombstoning.
    pub const fn bank_owner(&self) -> Option<OwnerToken> {
        self.bank_owner
    }

    /// Owner with valid recoverable live globals, independent of attribution.
    pub const fn resident_owner(&self) -> Option<OwnerToken> {
        self.resident
    }

    /// Read metadata only; this does not admit an engine read.
    ///
    /// # Errors
    /// Rejects foreign or unissued owner identities.
    pub fn owner_state(&self, owner: OwnerToken) -> Result<OwnerState, OwnerFault> {
        Ok(self.record(owner)?.state)
    }

    /// Obtain immutable checkpoint authority, not a snapshot of current globals.
    ///
    /// # Errors
    /// Rejects foreign, unissued, disposed, and faulted owners.
    pub fn checkpoint(&self, owner: OwnerToken) -> Result<Option<Arc<Checkpoint<C>>>, OwnerFault> {
        let record = self.record(owner)?;
        record.open()?;
        Ok(record.checkpoint.clone())
    }
}

#[cfg(test)]
mod tests;
