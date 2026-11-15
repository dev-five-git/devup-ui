use std::sync::Arc;

use super::identity::Key;
use super::{Checkpoint, CheckpointStamp, ExportTicket, OwnerFault, OwnerState, OwnerToken};

pub(super) struct OwnerRecord<C> {
    pub state: OwnerState,
    pub checkpoint: Option<Arc<Checkpoint<C>>>,
    pub clear: bool,
}

impl<C> OwnerRecord<C> {
    pub const fn new() -> Self {
        Self {
            state: OwnerState::Open,
            checkpoint: None,
            clear: false,
        }
    }

    pub const fn open(&self) -> Result<(), OwnerFault> {
        match self.state {
            OwnerState::Open => Ok(()),
            OwnerState::Faulted(fault) => Err(fault),
            OwnerState::Disposed => Err(OwnerFault::ClosedOwner),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Pending {
    Run(OwnerToken),
    Mutate,
    Clear(OwnerToken),
}

#[derive(Clone, Copy)]
pub(super) enum Phase {
    Idle,
    Preserving {
        key: Key,
        stamp: CheckpointStamp,
        next: Pending,
    },
    Activating {
        key: Key,
        owner: OwnerToken,
    },
    Configuring {
        key: Key,
        owner: OwnerToken,
    },
    Running {
        key: Key,
        owner: OwnerToken,
    },
    Clearing {
        key: Key,
        owner: OwnerToken,
    },
}

impl Phase {
    pub const fn key(self) -> Option<Key> {
        match self {
            Self::Idle => None,
            Self::Preserving { key, .. }
            | Self::Activating { key, .. }
            | Self::Configuring { key, .. }
            | Self::Running { key, .. }
            | Self::Clearing { key, .. } => Some(key),
        }
    }

    pub fn involves(self, owner: OwnerToken) -> bool {
        match self {
            Self::Idle => false,
            Self::Preserving { stamp, next, .. } => {
                stamp.owner == owner
                    || match next {
                        Pending::Run(target) | Pending::Clear(target) => target == owner,
                        Pending::Mutate => false,
                    }
            }
            Self::Activating { owner: target, .. }
            | Self::Configuring { owner: target, .. }
            | Self::Running { owner: target, .. }
            | Self::Clearing { owner: target, .. } => target == owner,
        }
    }
}

pub(super) const fn export_key(ticket: Option<ExportTicket>) -> Option<Key> {
    match ticket {
        Some(ticket) => Some(ticket.0),
        None => None,
    }
}
