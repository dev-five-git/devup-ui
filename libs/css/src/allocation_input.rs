//! Captured inputs for dormant legacy naming, independent of sheet expansion evidence.

use serde::{Deserialize, Serialize};

use crate::CounterOwner;

/// Exact naming input before effective declaration expansion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyInput {
    /// A declaration's legacy naming dimensions.
    Declaration(LegacyDeclaration),
    /// The exact old decimal hash/string supplied to keyframe naming.
    Keyframes(String),
    /// A no-site variable's legacy naming dimensions.
    Variable(LegacyVariable),
}

/// Authored declaration dimensions, not an emitted expansion witness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyDeclaration {
    pub property: String,
    pub level: u8,
    pub value: Option<String>,
    pub selector: Option<String>,
    pub order: Option<u8>,
}

/// Legacy no-site variable input; authored source-site variables are separate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyVariable {
    pub property: String,
    pub level: u8,
    pub selector: Option<String>,
}

/// Baseline naming branch, captured before pure projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NameMode {
    Counter,
    Debug,
    AtomHoist,
}

/// Captured naming configuration; serialized values confer no build authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedNameConfig {
    pub prefix: String,
    pub mode: NameMode,
}

/// Allocation identity, deliberately separate from canonical delivery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationFile {
    /// Low-level #751 filename namespace and its already resolved ordinal.
    Legacy { filename: String, ordinal: usize },
    /// Explicit original numeric authority supplied by a future extractor adapter.
    Original(u32),
}

impl AllocationFile {
    pub(crate) fn namespace(&self) -> String {
        match self {
            Self::Legacy { filename, .. } => filename.clone(),
            Self::Original(id) => format!("D9-{id}"),
        }
    }

    pub(crate) fn ordinal(&self) -> usize {
        match self {
            Self::Legacy { ordinal, .. } => *ordinal,
            Self::Original(id) => id
                .to_be_bytes()
                .into_iter()
                .fold(0usize, |number, byte| (number << 8) | usize::from(byte)),
        }
    }
}

/// Frozen delivery inputs used only to reproduce baseline atom scopes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedDelivery {
    pub canonical: String,
    pub hoisted: bool,
}

/// Complete map-independent inputs for key/name projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationContext {
    pub config: CapturedNameConfig,
    pub file: Option<AllocationFile>,
    pub delivery: Option<CapturedDelivery>,
}

/// Capture live producer inputs once; admission must use the resulting frozen data.
///
/// Only the inactive/unnumbered low-level branch resolves a legacy file ordinal.
/// Original ownership never reads or falls back to canonical delivery numbering.
#[must_use]
pub fn capture_context(
    input: &LegacyInput,
    filename: Option<&str>,
    owner: CounterOwner,
) -> AllocationContext {
    let _admission = crate::admission::enter();
    let mode = if crate::atom_hoist::is_atom_hoist() {
        NameMode::AtomHoist
    } else if crate::debug::is_debug() {
        NameMode::Debug
    } else {
        NameMode::Counter
    };
    let filename = match input {
        LegacyInput::Declaration(declaration) if declaration.order == Some(0) => None,
        LegacyInput::Declaration(_) | LegacyInput::Keyframes(_) => filename,
        LegacyInput::Variable(_) => None,
    };
    let needs_ordinal = match (mode, input) {
        (NameMode::Counter, _) | (NameMode::Debug, LegacyInput::Declaration(_)) => true,
        (NameMode::Debug, LegacyInput::Keyframes(_) | LegacyInput::Variable(_))
        | (NameMode::AtomHoist, _) => false,
    };
    let file = filename.and_then(|filename| match owner {
        CounterOwner::D9(id) => Some(AllocationFile::Original(id)),
        CounterOwner::Inactive | CounterOwner::Unnumbered => {
            needs_ordinal.then(|| AllocationFile::Legacy {
                filename: filename.to_string(),
                ordinal: crate::file_map::get_file_num_by_filename(filename),
            })
        }
    });
    let delivery = filename.map(|filename| CapturedDelivery {
        canonical: crate::file_map::canonical(filename),
        hoisted: mode == NameMode::AtomHoist
            && matches!(input, LegacyInput::Declaration(_))
            && crate::atom_hoist::is_hoisted_bucket(filename),
    });
    AllocationContext {
        config: CapturedNameConfig {
            prefix: crate::get_prefix().unwrap_or_default(),
            mode,
        },
        file,
        delivery,
    }
}

pub(crate) fn effective_file<'a>(
    input: &LegacyInput,
    context: &'a AllocationContext,
) -> Option<&'a AllocationFile> {
    match input {
        LegacyInput::Declaration(declaration) if declaration.order == Some(0) => None,
        LegacyInput::Declaration(_) | LegacyInput::Keyframes(_) => context.file.as_ref(),
        LegacyInput::Variable(_) => None,
    }
}
