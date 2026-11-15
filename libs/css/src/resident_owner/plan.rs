//! Owned plans release every coordinator borrow before host work begins.

use std::sync::Arc;

use super::{
    ActivationTicket, CheckpointStamp, CheckpointTicket, ClearTicket, ConfigurationTicket,
    ExportTicket,
};

/// Complete immutable capture. `C` must be an immutable value, not a live engine view.
#[derive(Debug)]
pub struct Checkpoint<C> {
    pub(super) stamp: CheckpointStamp,
    pub(super) payload: C,
}

impl<C> Checkpoint<C> {
    /// Authority attached at the single successful capture commit.
    pub const fn stamp(&self) -> CheckpointStamp {
        self.stamp
    }

    /// Borrow the immutable captured value; never the current physical bank.
    pub const fn payload(&self) -> &C {
        &self.payload
    }
}

/// Request to capture the outgoing owner before any destructive work.
#[derive(Debug, Eq, PartialEq)]
pub struct PreservationPlan {
    /// Only this ticket may commit or abort the capture.
    pub ticket: CheckpointTicket,
    /// Identifies the exact still-live outgoing state.
    pub stamp: CheckpointStamp,
}

/// Request to reset, then restore the optional checkpoint, before configuration.
#[derive(Debug)]
pub struct ActivationPlan<C> {
    /// Complete only after the entire reset/restore succeeds.
    pub ticket: ActivationTicket,
    /// Owned immutable payload; no coordinator borrow crosses host work.
    pub restore: Option<Arc<Checkpoint<C>>>,
}

/// Exact next managed-run step, never a claim that engine work was performed.
#[derive(Debug)]
pub enum RunPlan<C> {
    /// Capture before requesting reset/restore.
    Preserve(PreservationPlan),
    /// Perform reset/restore before requesting configuration.
    Activate(ActivationPlan<C>),
    /// Replay configuration, then call `configuration_completed` to admit action.
    Configure(ConfigurationTicket),
}

/// Next public-export step. Admission precedes all host argument conversion.
#[derive(Debug, Eq, PartialEq)]
pub enum ExportPlan {
    /// Unmanaged mutation must first preserve the current resident.
    Preserve(PreservationPlan),
    /// Invoke the export, retaining this ticket through return/error processing.
    Admitted(ExportTicket),
}

/// Next owner-targeted cleanup step. No variant performs a physical reset.
#[derive(Debug, Eq, PartialEq)]
pub enum ClearPlan {
    /// Preserve the open resident before resetting its attributed bank.
    Preserve(PreservationPlan),
    /// Perform a targeted reset, then complete this ticket.
    Reset(ClearTicket),
    /// Drain using `poll_clear` after active frames unwind.
    Deferred,
    /// The bank belongs to someone else, or there is no pending clear.
    Skipped,
}

/// Continuation authorized only by a matching completed outgoing capture.
#[derive(Debug)]
pub enum ResumePlan<C> {
    /// Continue a managed run.
    Run(RunPlan<C>),
    /// Admit the unmanaged mutation.
    Export(ExportPlan),
    /// Continue targeted cleanup.
    Clear(ClearPlan),
}

/// Snapshot exports permitted sequentially within an authorized preservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreservationKind {
    /// Complete sheet including the current frozen atom plan.
    Sheet,
    /// Shared class/variable/keyframe allocator maps.
    ClassMap,
    /// File/preseed inventory.
    FileMap,
    /// Canonical mapping.
    CanonicalMap,
    /// Opaque reservation bookkeeping.
    ReservationState,
}

/// Classification supplied by the binding's fixed public export inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportKind {
    /// Guarded non-mutating engine read, including resolver-bearing reads.
    Read,
    /// Any potentially mutating call, including argument-conversion failures.
    Mutate,
    /// Allowlisted snapshot during the matching capture, not an arbitrary read.
    Preservation(CheckpointTicket, PreservationKind),
}

/// Exact transition authority used for failure unwind, never an action ticket.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionTicket {
    /// Outgoing capture.
    Preservation(CheckpointTicket),
    /// Incoming reset/restore.
    Activation(ActivationTicket),
    /// Configuration replay.
    Configuration(ConfigurationTicket),
    /// Targeted reset.
    Clear(ClearTicket),
}
