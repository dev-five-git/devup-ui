use super::FrozenAuthority;
use crate::counter_evidence::{AllocationEvidence, ReplayError};
use css::{
    allocation_input::{AllocationContext, LegacyInput},
    counter_names::{NameAddress, allocation_key, render_name},
};

impl FrozenAuthority {
    pub(super) fn envelope(
        &self,
        evidence: &AllocationEvidence,
        input: &LegacyInput,
        context: &AllocationContext,
    ) -> Result<(), ReplayError> {
        if &evidence.input != input || &evidence.context != context {
            return Err(ReplayError::Allocation);
        }
        let slot = match (&evidence.allocation.address, allocation_key(input, context)) {
            (
                NameAddress::Counter {
                    namespace,
                    legacy_key,
                    slot,
                },
                Some((expected_namespace, expected_key)),
            ) => {
                if namespace != &expected_namespace
                    || legacy_key != &expected_key
                    || self
                        .classes
                        .get(namespace)
                        .and_then(|map| map.get(legacy_key))
                        != Some(slot)
                {
                    return Err(ReplayError::Allocation);
                }
                *slot
            }
            (
                NameAddress::Baseline {
                    mode,
                    input: captured_input,
                    context: captured_context,
                },
                None,
            ) => {
                if *mode != context.config.mode
                    || captured_input != input
                    || captured_context != context
                {
                    return Err(ReplayError::Allocation);
                }
                0
            }
            (NameAddress::Counter { .. }, None) | (NameAddress::Baseline { .. }, Some(_)) => {
                return Err(ReplayError::Allocation);
            }
        };
        if render_name(input, context, slot) != evidence.allocation.name {
            return Err(ReplayError::Allocation);
        }
        Ok(())
    }
}
