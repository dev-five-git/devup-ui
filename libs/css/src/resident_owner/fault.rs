//! Typed protocol refusals; host error mapping belongs to the binding.

use std::fmt;

/// Checked monotonic authority space that cannot issue another value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Counter {
    /// Coordinator identities.
    Engine,
    /// Owner identities within a coordinator.
    Owner,
    /// Operation identities within a coordinator.
    Ticket,
    /// Physical mutation epochs.
    Epoch,
}

/// Failed host transition phase; action errors are not transition failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionPhase {
    /// Whole outgoing capture failed.
    Preservation,
    /// Reset or restore failed.
    Activation,
    /// Configuration replay failed.
    Configuration,
    /// Targeted reset failed.
    Clear,
}

/// Structural ownership fault. The first refusal is latched until unwind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerFault {
    /// Authority came from a different coordinator.
    ForeignEngine,
    /// No owner was issued with this identity.
    InvalidOwner,
    /// The owner was tombstoned.
    ClosedOwner,
    /// Ticket is no longer the current operation.
    StaleTicket,
    /// A synchronous operation attempted forbidden reentry.
    Reentry,
    /// Completion does not match the currently authorized phase.
    WrongPhase,
    /// Deferred targeted cleanup must drain before new work.
    CleanupPending,
    /// Capture failed; no older checkpoint can recover newer live state.
    CaptureFailed,
    /// Reset/restore failed; action was not admitted.
    ActivationFailed,
    /// Configuration failed; action was not admitted.
    ConfigurationFailed,
    /// Targeted reset failed.
    CleanupFailed,
    /// A checked authority space was exhausted.
    Exhausted(Counter),
}

impl fmt::Display for OwnerFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "resident ownership fault: {self:?}")
    }
}

impl std::error::Error for OwnerFault {}
