use css::{
    allocation_input::{AllocationContext, LegacyInput, NameMode},
    counter_names::{AllocatedName, NameAddress, allocation_key, render_name},
};
use serde::{Deserialize, Serialize};

use super::{EmissionWitness, RecordFootprint, ReplayError};

/// Producer-supplied serialized candidates, never current-build map authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationEvidence {
    pub input: LegacyInput,
    pub context: AllocationContext,
    pub allocation: AllocatedName,
}

impl AllocationEvidence {
    /// Check the captured envelope without reserving slots or consulting maps.
    pub fn check(&self) -> Result<(), ReplayError> {
        let slot = match &self.allocation.address {
            NameAddress::Counter {
                namespace,
                legacy_key,
                slot,
            } => {
                if allocation_key(&self.input, &self.context).as_ref()
                    != Some(&(namespace.clone(), legacy_key.clone()))
                {
                    return Err(ReplayError::Allocation);
                }
                *slot
            }
            NameAddress::Baseline {
                mode,
                input,
                context,
            } => {
                match mode {
                    NameMode::Counter => return Err(ReplayError::Allocation),
                    NameMode::Debug | NameMode::AtomHoist => {}
                }
                if *mode != self.context.config.mode
                    || input != &self.input
                    || context != &self.context
                {
                    return Err(ReplayError::Allocation);
                }
                0
            }
        };
        if render_name(&self.input, &self.context, slot) != self.allocation.name {
            return Err(ReplayError::Allocation);
        }
        Ok(())
    }
}

/// Candidate allocator envelope plus an independently replayable emission witness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpansionProof {
    pub allocation: AllocationEvidence,
    pub emission: EmissionWitness,
}

impl ExpansionProof {
    /// Check pure consistency only; does not prove classMap or seed naming input.
    pub fn new(
        allocation: AllocationEvidence,
        emission: EmissionWitness,
    ) -> Result<Self, ReplayError> {
        let candidate = Self {
            allocation,
            emission,
        };
        candidate.materialized()?;
        Ok(candidate)
    }

    /// Bind exact producer name to complete replay, then sanctioned cleanup.
    pub fn materialized(&self) -> Result<Vec<&RecordFootprint>, ReplayError> {
        self.allocation.check()?;
        if self.allocation.allocation.name != self.emission.name {
            return Err(ReplayError::Allocation);
        }
        self.emission.materialized()
    }
}
