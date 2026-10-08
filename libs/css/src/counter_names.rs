//! Dormant #751 allocation requests and map-independent proof projections.

use serde::{Deserialize, Serialize};

use crate::allocation_input::{
    AllocationContext, AllocationFile, LegacyInput, NameMode, effective_file,
};
use crate::counter_allocation::reserve_counter;
use crate::num_to_nm_base::num_to_nm_base;

/// A produced name with the address returned by its actual allocation request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocatedName {
    pub name: String,
    pub address: NameAddress,
}

/// Candidate allocation proof; neither serialization nor rendering admits a cache.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NameAddress {
    Counter {
        namespace: String,
        legacy_key: String,
        slot: usize,
    },
    Baseline {
        mode: NameMode,
        input: LegacyInput,
        context: AllocationContext,
    },
}

/// Request the real legacy key once, returning its actual stored numeric slot.
#[must_use]
pub fn allocate_name(input: &LegacyInput, context: &AllocationContext) -> AllocatedName {
    match allocation_key(input, context) {
        Some((namespace, legacy_key)) => {
            let slot = reserve_counter(&namespace, &legacy_key);
            AllocatedName {
                name: render_name(input, context, slot.index()),
                address: NameAddress::Counter {
                    namespace,
                    legacy_key,
                    slot: slot.index(),
                },
            }
        }
        None => AllocatedName {
            name: render_name(input, context, 0),
            address: NameAddress::Baseline {
                mode: context.config.mode,
                input: input.clone(),
                context: context.clone(),
            },
        },
    }
}

/// Reconstruct namespace/key using only captured inputs, without reserving a slot.
#[must_use]
pub fn allocation_key(
    input: &LegacyInput,
    context: &AllocationContext,
) -> Option<(String, String)> {
    match context.config.mode {
        NameMode::Debug | NameMode::AtomHoist => None,
        NameMode::Counter => {
            let file = effective_file(input, context);
            let namespace = file.map_or_else(String::new, AllocationFile::namespace);
            let key = match input {
                LegacyInput::Declaration(declaration) => {
                    let mut key = crate::counter_render::declaration_body(declaration, false);
                    if let Some(file) = file {
                        key.push('-');
                        key.push_str(&num_to_nm_base(file.ordinal()));
                    }
                    key
                }
                LegacyInput::Keyframes(keyframes) => format!("k-{keyframes}"),
                LegacyInput::Variable(variable) => {
                    crate::counter_render::variable_body(variable, false)
                }
            };
            Some((namespace, key))
        }
    }
}

/// Render a candidate slot without accessing maps or current configuration.
///
/// Baseline branches ignore `slot`. Admission must separately check the actual
/// namespace/key/slot and captured configuration against current-build authority.
#[must_use]
pub fn render_name(input: &LegacyInput, context: &AllocationContext, slot: usize) -> String {
    match context.config.mode {
        NameMode::AtomHoist => crate::counter_render::atom_name(input, context),
        NameMode::Debug => crate::counter_render::debug_name(input, context),
        NameMode::Counter => {
            let prefix = &context.config.prefix;
            let base = num_to_nm_base(slot);
            match input {
                LegacyInput::Variable(_) => format!("--{prefix}{base}"),
                LegacyInput::Declaration(_) | LegacyInput::Keyframes(_) => {
                    match effective_file(input, context) {
                        Some(file) => format!("{prefix}{}-{base}", num_to_nm_base(file.ordinal())),
                        None => format!("{prefix}{base}"),
                    }
                }
            }
        }
    }
}
