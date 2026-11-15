use std::collections::BTreeMap;

use css::resident_owner::{
    ConfigurationTicket, Counter, ExportPlan, ExportTicket, OwnerCoordinator, OwnerFault,
    OwnerToken, RunTicket,
};

use super::checkpoint::HostCheckpoint;

pub(super) enum Execution {
    Configure(ConfigurationTicket),
    Run(RunTicket),
}

pub(super) struct Bank {
    pub core: OwnerCoordinator<HostCheckpoint>,
    pub owners: BTreeMap<u64, OwnerToken>,
    pub execution: Option<Execution>,
    pub export: Option<(u64, ExportTicket)>,
    pub capture: Option<css::resident_owner::CheckpointTicket>,
    sequence: u64,
    fault: Option<OwnerFault>,
}

impl Bank {
    pub fn new() -> Result<Self, OwnerFault> {
        Ok(Self {
            core: OwnerCoordinator::new()?,
            owners: BTreeMap::new(),
            execution: None,
            export: None,
            capture: None,
            sequence: 0,
            fault: None,
        })
    }

    pub fn issue(&mut self) -> Result<u64, OwnerFault> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(OwnerFault::Exhausted(Counter::Ticket))?;
        Ok(self.sequence)
    }

    pub fn create(&mut self) -> Result<u64, OwnerFault> {
        let id = self.issue()?;
        let owner = self.core.create_owner()?;
        self.owners.insert(id, owner);
        Ok(id)
    }

    pub fn enter_export(&mut self, opcode: u32) -> Result<(ExportPlan, u64), OwnerFault> {
        let result = self.issue().and_then(|id| {
            self.check_fault()
                .and_then(|()| super::protocol::kind(opcode, self.capture))
                .and_then(|kind| self.core.begin_export(kind))
                .map(|plan| (plan, id))
        });
        self.latch(result)
    }

    pub fn owner(&self, id: u64) -> Result<OwnerToken, OwnerFault> {
        self.owners
            .get(&id)
            .copied()
            .ok_or(OwnerFault::InvalidOwner)
    }

    pub fn dispose(&mut self, id: u64) -> Result<(), OwnerFault> {
        let owner = self.owner(id)?;
        self.core.dispose(owner)?;
        if self.core.bank_owner() == Some(owner)
            && (self.execution.is_some() || self.export.is_some() || self.capture.is_some())
        {
            self.fault.get_or_insert(OwnerFault::ClosedOwner);
        }
        Ok(())
    }

    pub const fn check_fault(&self) -> Result<(), OwnerFault> {
        match self.fault {
            Some(fault) => Err(fault),
            None => Ok(()),
        }
    }

    pub fn latch<T>(&mut self, result: Result<T, OwnerFault>) -> Result<T, OwnerFault> {
        if let Err(fault) = result
            && (self.execution.is_some() || self.export.is_some() || self.capture.is_some())
        {
            self.fault.get_or_insert(fault);
        }
        result
    }

    pub const fn take_fault(&mut self) -> Result<(), OwnerFault> {
        match self.fault.take() {
            Some(fault) => Err(fault),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "transport_limit_tests.rs"]
mod transport_limit_tests;
