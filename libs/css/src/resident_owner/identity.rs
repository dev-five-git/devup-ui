//! Authority values have private constructors and never recycle a sequence.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{Counter, OwnerFault};

static ENGINES: AtomicUsize = AtomicUsize::new(0);

/// Identity of one coordinator for one physical state bank.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EngineId(pub(super) usize);

impl EngineId {
    pub(super) fn allocate(source: &AtomicUsize) -> Result<Self, OwnerFault> {
        source
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map(Self)
            .map_err(|_| OwnerFault::Exhausted(Counter::Engine))
    }

    pub(super) fn fresh() -> Result<Self, OwnerFault> {
        Self::allocate(&ENGINES)
    }
}

/// Opaque owner identity, scoped to its originating coordinator.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OwnerToken {
    pub(super) engine: EngineId,
    pub(super) sequence: u64,
}

/// Monotonic physical-bank mutation epoch, never restored from a checkpoint.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Epoch(pub(super) u64);

/// Exact physical state captured by a completed checkpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckpointStamp {
    pub(super) owner: OwnerToken,
    pub(super) epoch: Epoch,
}

impl CheckpointStamp {
    /// Originating physical engine identity.
    pub const fn engine(self) -> EngineId {
        self.owner.engine
    }

    /// Owner whose entire state was captured.
    pub const fn owner(self) -> OwnerToken {
        self.owner
    }

    /// Epoch at which capture was authorized.
    pub const fn epoch(self) -> Epoch {
        self.epoch
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Key {
    pub engine: EngineId,
    pub sequence: u64,
}

/// Authority to capture an outgoing owner, not to mutate the bank.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckpointTicket(pub(super) Key);

/// Authority to perform exactly the planned reset/restore activation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationTicket(pub(super) Key);

/// Authority to replay configuration, before action admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigurationTicket(pub(super) Key);

/// Authority for one synchronous managed action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunTicket(pub(super) Key);

/// Authority covering marshalling, an export, and all its return processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExportTicket(pub(super) Key);

/// Authority to reset the bank attributed to a particular clear request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClearTicket(pub(super) Key);

pub(super) fn advance(value: &mut u64, counter: Counter) -> Result<u64, OwnerFault> {
    let next = value.checked_add(1).ok_or(OwnerFault::Exhausted(counter))?;
    *value = next;
    Ok(next)
}
