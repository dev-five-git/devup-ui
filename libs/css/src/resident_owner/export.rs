use super::state::{Pending, Phase, export_key};
use super::{ExportKind, ExportPlan, ExportTicket, OwnerCoordinator, OwnerFault};

impl<C> OwnerCoordinator<C> {
    /// Admit an inventoried export before marshalling, getters, or resolver calls.
    ///
    /// # Errors
    /// Refuses nested reads/mutations and wrong preservation authority, latching
    /// the refusal on the enclosing operation. Only sequential allowlisted
    /// preservation exports may enter during capture. Mutators advance epoch
    /// before admission even if later argument conversion or execution fails.
    pub fn begin_export(&mut self, kind: ExportKind) -> Result<ExportPlan, OwnerFault> {
        self.exports_unwound()?;
        self.unlatched()?;
        match self.phase {
            Phase::Idle => {
                self.cleanup_drained()?;
                match kind {
                    ExportKind::Read => self.admit_read().map(ExportPlan::Admitted),
                    ExportKind::Mutate => match self.preserve(Pending::Mutate)? {
                        Some(plan) => Ok(ExportPlan::Preserve(plan)),
                        None => self.admit_mutation().map(ExportPlan::Admitted),
                    },
                    ExportKind::Preservation(..) => self.refuse(OwnerFault::WrongPhase),
                }
            }
            Phase::Preserving { key, .. } => match kind {
                ExportKind::Preservation(ticket, _) => {
                    self.check_key(ticket.0, Some(key))?;
                    self.admit_read().map(ExportPlan::Admitted)
                }
                ExportKind::Read | ExportKind::Mutate => self.refuse(OwnerFault::Reentry),
            },
            Phase::Configuring { owner, .. } | Phase::Running { owner, .. } => {
                self.require_open(owner)?;
                match kind {
                    ExportKind::Read => self.admit_read().map(ExportPlan::Admitted),
                    ExportKind::Mutate => self.admit_mutation().map(ExportPlan::Admitted),
                    ExportKind::Preservation(..) => self.refuse(OwnerFault::Reentry),
                }
            }
            Phase::Activating { .. } | Phase::Clearing { .. } => self.refuse(OwnerFault::Reentry),
        }
    }

    fn admit_read(&mut self) -> Result<ExportTicket, OwnerFault> {
        let ticket = ExportTicket(self.issue_key()?);
        self.export = Some(ticket);
        Ok(ticket)
    }

    pub(super) fn admit_mutation(&mut self) -> Result<ExportTicket, OwnerFault> {
        let ticket = ExportTicket(self.issue_key()?);
        self.mutate()?;
        match self.phase {
            Phase::Configuring { owner, .. } | Phase::Running { owner, .. } => {
                let record = self
                    .owners
                    .get_mut(&owner)
                    .ok_or(OwnerFault::InvalidOwner)?;
                record.checkpoint = None;
            }
            Phase::Idle
            | Phase::Preserving { .. }
            | Phase::Activating { .. }
            | Phase::Clearing { .. } => {
                self.resident = None;
                self.bank_owner = None;
            }
        }
        self.export = Some(ticket);
        Ok(ticket)
    }

    /// Unwind the exact export after all host return/error processing finishes.
    ///
    /// # Errors
    /// Foreign/stale tickets cannot release another export. A matching ticket
    /// releases this frame even when returning a latched fault; managed-run or
    /// preservation latches remain until their enclosing ticket also unwinds.
    pub fn end_export(&mut self, ticket: ExportTicket) -> Result<(), OwnerFault> {
        self.check_key(ticket.0, export_key(self.export))?;
        self.export = None;
        if self.phase.key().is_none() {
            match self.fault.take() {
                Some(fault) => Err(fault),
                None => Ok(()),
            }
        } else {
            self.unlatched()
        }
    }
}
