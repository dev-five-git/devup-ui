use super::*;

mod edges;
mod exports;
mod failures;
mod lifecycle;
mod limits;
mod runs;
mod tickets;
mod unwind;

type Core = OwnerCoordinator<Vec<u64>>;
type TestResult = Result<(), OwnerFault>;

fn start(core: &mut Core, owner: OwnerToken) -> Result<RunTicket, OwnerFault> {
    let configuration = match core.begin_run(owner)? {
        RunPlan::Activate(plan) => core.activation_completed(plan.ticket)?,
        RunPlan::Configure(ticket) => ticket,
        RunPlan::Preserve(_) => panic!("fixture must explicitly complete takeover preservation"),
    };
    core.configuration_completed(configuration)
}

fn resident() -> Result<(Core, OwnerToken), OwnerFault> {
    let mut core = Core::new()?;
    let owner = core.create_owner()?;
    let ticket = start(&mut core, owner)?;
    core.end_run(ticket)?;
    Ok((core, owner))
}

fn admitted(plan: ExportPlan) -> ExportTicket {
    match plan {
        ExportPlan::Admitted(ticket) => ticket,
        ExportPlan::Preserve(_) => panic!("fixture must explicitly complete mutation preservation"),
    }
}

fn takeover(core: &mut Core, owner: OwnerToken) -> Result<PreservationPlan, OwnerFault> {
    match core.begin_run(owner)? {
        RunPlan::Preserve(plan) => Ok(plan),
        RunPlan::Activate(_) | RunPlan::Configure(_) => panic!("expected outgoing preservation"),
    }
}

fn resume_run(plan: ResumePlan<Vec<u64>>) -> ActivationPlan<Vec<u64>> {
    match plan {
        ResumePlan::Run(RunPlan::Activate(plan)) => plan,
        ResumePlan::Run(RunPlan::Preserve(_) | RunPlan::Configure(_))
        | ResumePlan::Export(_)
        | ResumePlan::Clear(_) => panic!("expected takeover activation"),
    }
}
