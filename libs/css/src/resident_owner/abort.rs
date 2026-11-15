use super::identity::Key;
use super::state::Phase;
use super::{OwnerCoordinator, OwnerFault, OwnerState, TransitionPhase, TransitionTicket};

impl TransitionTicket {
    const fn parts(self) -> (Key, TransitionPhase) {
        match self {
            Self::Preservation(ticket) => (ticket.0, TransitionPhase::Preservation),
            Self::Activation(ticket) => (ticket.0, TransitionPhase::Activation),
            Self::Configuration(ticket) => (ticket.0, TransitionPhase::Configuration),
            Self::Clear(ticket) => (ticket.0, TransitionPhase::Clear),
        }
    }
}

impl<C> OwnerCoordinator<C> {
    /// Unwind a failed transition, retaining every completed outgoing checkpoint.
    ///
    /// # Errors
    /// A matching abort unwinds and returns the first latched refusal or the typed
    /// phase failure. A foreign/stale/wrong-phase ticket or live export refuses
    /// without unwinding. A failed capture faults the outgoing owner and discards
    /// older checkpoint authority; activation/configuration faults only incoming.
    pub fn abort_transition(
        &mut self,
        ticket: TransitionTicket,
        phase: TransitionPhase,
    ) -> Result<(), OwnerFault> {
        let (key, ticket_phase) = ticket.parts();
        let (owner, actual, failure) = match self.phase {
            Phase::Preserving { stamp, .. } => (
                stamp.owner,
                TransitionPhase::Preservation,
                OwnerFault::CaptureFailed,
            ),
            Phase::Activating { owner, .. } => (
                owner,
                TransitionPhase::Activation,
                OwnerFault::ActivationFailed,
            ),
            Phase::Configuring { owner, .. } => (
                owner,
                TransitionPhase::Configuration,
                OwnerFault::ConfigurationFailed,
            ),
            Phase::Clearing { owner, .. } => {
                (owner, TransitionPhase::Clear, OwnerFault::CleanupFailed)
            }
            Phase::Idle | Phase::Running { .. } => return self.reject_ticket(key),
        };
        self.check_key(key, self.phase.key())?;
        self.exports_unwound()?;
        if ticket_phase != phase || actual != phase {
            return self.refuse(OwnerFault::WrongPhase);
        }
        self.resident = None;
        match phase {
            TransitionPhase::Preservation
            | TransitionPhase::Activation
            | TransitionPhase::Configuration => {
                let record = self
                    .owners
                    .get_mut(&owner)
                    .ok_or(OwnerFault::InvalidOwner)?;
                match record.state {
                    OwnerState::Open | OwnerState::Faulted(_) => {
                        record.state = OwnerState::Faulted(failure);
                    }
                    OwnerState::Disposed => {}
                }
                record.checkpoint = None;
            }
            TransitionPhase::Clear => {}
        }
        self.phase = Phase::Idle;
        Err(self.fault.take().unwrap_or(failure))
    }
}
