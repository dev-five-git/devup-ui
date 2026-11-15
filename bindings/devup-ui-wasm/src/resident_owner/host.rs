use css::resident_owner::{
    OwnerFault, PreservationPlan, ResumePlan, TransitionPhase, TransitionTicket,
};

use super::{
    bank::Bank,
    checkpoint::HostCheckpoint,
    decision,
    interpreter::{ErrorTransport, Owner},
};

impl<E: ErrorTransport, F: Clone, T: Into<E::Value>> Owner<'_, E, F, T> {
    pub(super) fn create(&self, capture: F) -> Result<u64, E::Value> {
        let id = self
            .bank
            .with(Bank::create)
            .map_err(|fault| self.errors.error(fault))?;
        self.callbacks
            .with_borrow_mut(|callbacks| callbacks.insert(id, capture));
        Ok(id)
    }

    pub(super) fn reject(&self) -> Result<(), E::Value> {
        self.bank
            .with(|bank| bank.latch(Err(OwnerFault::ForeignEngine)))
            .map_err(|fault| self.errors.error(fault))
    }

    pub(super) fn abort(
        &self,
        ticket: TransitionTicket,
        phase: TransitionPhase,
        primary: E::Value,
    ) -> E::Value {
        let outcome = self
            .bank
            .with(|bank| bank.core.abort_transition(ticket, phase));
        self.aborted(outcome, phase, primary)
    }

    pub(super) fn aborted(
        &self,
        outcome: Result<(), OwnerFault>,
        phase: TransitionPhase,
        primary: E::Value,
    ) -> E::Value {
        let failure = self
            .errors
            .with_cause(decision::phase_fault(phase), primary);
        match outcome {
            Err(fault) if fault == decision::phase_fault(phase) => failure,
            Err(fault) => self.errors.cleanup(failure, self.errors.error(fault)),
            Ok(()) => self
                .errors
                .cleanup(failure, self.errors.error(OwnerFault::WrongPhase)),
        }
    }

    pub(super) fn preserve(
        &self,
        plan: PreservationPlan,
    ) -> Result<ResumePlan<HostCheckpoint>, E::Value> {
        let ticket = plan.ticket;
        let id = self
            .bank
            .with(|bank| {
                let id = bank
                    .owners
                    .iter()
                    .find_map(|(id, owner)| (*owner == plan.stamp.owner()).then_some(*id))
                    .ok_or(OwnerFault::InvalidOwner)?;
                bank.capture = Some(ticket);
                Ok(id)
            })
            .map_err(|fault| self.errors.error(fault))?;
        let capture = self
            .callbacks
            .with_borrow(|callbacks| callbacks.get(&id).cloned());
        let captured = match capture {
            Some(capture) => (self.call.invoke)(&capture, &self.call.receiver)
                .and_then(|value| self.decode(value)),
            None => Err(self.errors.error(OwnerFault::ClosedOwner)),
        };
        let result = match captured {
            Err(primary) => Err(self.abort(
                TransitionTicket::Preservation(ticket),
                TransitionPhase::Preservation,
                primary,
            )),
            Ok(payload) => match self.bank.with(|bank| {
                let completion = bank
                    .check_fault()
                    .and_then(|()| bank.core.checkpoint_completed(ticket, payload));
                Ok(completion
                    .map_err(|fault| decision::capture_failure(&mut bank.core, &plan, fault)))
            }) {
                Ok(Ok(next)) => Ok(next),
                Ok(Err(decision::CaptureFailure::Committed(fault))) | Err(fault) => {
                    Err(self.errors.error(fault))
                }
                Ok(Err(decision::CaptureFailure::Aborted { primary, unwind })) => Err(self
                    .aborted(
                        unwind,
                        TransitionPhase::Preservation,
                        self.errors.error(primary),
                    )),
            },
        };
        let unwind = self
            .bank
            .with(|bank| {
                bank.capture = None;
                bank.take_fault()
            })
            .map_err(|fault| self.errors.error(fault));
        self.errors.complete(result, unwind)
    }

    pub(super) fn decode(&self, value: E::Value) -> Result<HostCheckpoint, E::Value> {
        let field = |name: &str| {
            let value = (self.decoder.get)(&value, &(self.decoder.string)(name))?;
            (self.decoder.as_string)(&value).ok_or_else(|| {
                (self.decoder.type_error)("checkpoint fields must be strings").into()
            })
        };
        let parts = [
            field("sheet")?,
            field("classes")?,
            field("files")?,
            field("canonical")?,
            field("reservation")?,
        ];
        HostCheckpoint::parse(parts.each_ref().map(String::as_str))
            .map_err(|error| self.errors.message(&error.to_string()))
    }
}
