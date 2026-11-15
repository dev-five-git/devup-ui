use css::resident_owner::{
    OwnerCoordinator, OwnerFault, PreservationPlan, TransitionPhase, TransitionTicket,
};

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Faults {
    pub primary: OwnerFault,
    pub cleanup: Option<OwnerFault>,
}

pub(super) const fn phase_fault(phase: TransitionPhase) -> OwnerFault {
    match phase {
        TransitionPhase::Preservation => OwnerFault::CaptureFailed,
        TransitionPhase::Activation => OwnerFault::ActivationFailed,
        TransitionPhase::Configuration => OwnerFault::ConfigurationFailed,
        TransitionPhase::Clear => OwnerFault::CleanupFailed,
    }
}

pub(super) fn unwind(
    result: Result<(), OwnerFault>,
    latched: Result<(), OwnerFault>,
    expected_abort: Option<OwnerFault>,
) -> Result<(), Faults> {
    match (result, latched) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) | (Ok(()), Err(primary)) => Err(Faults {
            primary,
            cleanup: None,
        }),
        (Err(completion), Err(primary)) => Err(Faults {
            primary,
            cleanup: (completion != primary && Some(completion) != expected_abort)
                .then_some(completion),
        }),
    }
}

pub(super) enum CaptureFailure {
    Committed(OwnerFault),
    Aborted {
        primary: OwnerFault,
        unwind: Result<(), OwnerFault>,
    },
}

pub(super) fn capture_failure<C>(
    core: &mut OwnerCoordinator<C>,
    plan: &PreservationPlan,
    fault: OwnerFault,
) -> CaptureFailure {
    let committed = matches!(
        core.checkpoint(plan.stamp.owner()),
        Ok(Some(checkpoint)) if checkpoint.stamp() == plan.stamp
    );
    if committed {
        CaptureFailure::Committed(fault)
    } else {
        CaptureFailure::Aborted {
            primary: fault,
            unwind: core.abort_transition(
                TransitionTicket::Preservation(plan.ticket),
                TransitionPhase::Preservation,
            ),
        }
    }
}
