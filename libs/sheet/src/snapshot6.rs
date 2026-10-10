//! Dormant strict snapshot admission; production cache4 dispatch is unchanged.
use super::{KernelError, state_live};
use crate::{
    StyleSheet,
    cache_snapshot::{ClassMap, FileMap},
};
use serde::Serialize;

#[path = "snapshot6_admission.rs"]
mod admission;
#[path = "snapshot6_allocation.rs"]
mod allocation;
#[path = "snapshot6_evidence.rs"]
mod evidence;
#[path = "snapshot6_install.rs"]
mod install;
#[path = "snapshot6_raw.rs"]
mod raw;
#[path = "snapshot6_records.rs"]
mod records;
#[path = "snapshot6_seed.rs"]
mod seed;
#[path = "snapshot6_selector.rs"]
mod selector;
#[path = "snapshot6_sheet.rs"]
mod sheet;
#[path = "snapshot6_wire.rs"]
mod wire;
use wire::Wire;

#[cfg(test)]
#[path = "snapshot6_adoption_tests.rs"]
mod adoption_tests;
#[cfg(test)]
#[path = "snapshot6_advanced_schema_tests.rs"]
mod advanced_schema_tests;
#[cfg(test)]
#[path = "snapshot6_advanced_set_tests.rs"]
mod advanced_set_tests;
#[cfg(test)]
#[path = "snapshot6_advanced_support.rs"]
mod advanced_support;
#[cfg(test)]
#[path = "snapshot6_boundary_tests.rs"]
mod boundary_tests;
#[cfg(test)]
#[path = "snapshot6_cleanup_tests.rs"]
mod cleanup_tests;
#[cfg(test)]
#[path = "snapshot6_codec_tests.rs"]
mod codec_tests;
#[cfg(test)]
#[path = "snapshot6_coordinate_tests.rs"]
mod coordinate_tests;
#[cfg(test)]
#[path = "snapshot6_coverage_tests.rs"]
mod coverage_tests;
#[cfg(test)]
#[path = "snapshot6_damage_tests.rs"]
mod damage_tests;
#[cfg(test)]
#[path = "snapshot6_lifecycle_tests.rs"]
mod lifecycle_tests;
#[cfg(test)]
#[path = "snapshot6_multiplicity_tests.rs"]
mod multiplicity_tests;
#[cfg(test)]
#[path = "snapshot6_placement_tests.rs"]
mod placement_tests;
#[cfg(test)]
#[path = "snapshot6_replay_tests.rs"]
mod replay_tests;
#[cfg(test)]
#[path = "snapshot6_schema_tests.rs"]
mod schema_tests;
#[cfg(test)]
#[path = "snapshot6_test_support.rs"]
mod test_support;

/// A strict schema, provenance, configuration or installation rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    /// Required raw shape or multiplicity disagrees.
    Schema,
    /// A full registry is not independently dense and unique.
    Map,
    /// No explicitly successful Counter state exists.
    State,
    /// Actual current naming configuration or frozen plan disagrees.
    Configuration,
    /// Frozen cleanup history is internally inconsistent.
    Cleanup,
    /// Supplied companion differs from the snapshot's full registry.
    Companion,
    /// Existing linkage rejected; its diagnostic precision is preserved.
    Kernel(KernelError),
    /// Installation cannot run during an exact attempt.
    ActiveExact(css::admission::ActiveExactAttempt),
}
impl std::fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "strict snapshot6: {self:?}")
    }
}
impl std::error::Error for EvidenceError {}

struct Authority {
    sheet: StyleSheet,
    classes: ClassMap,
    files: FileMap,
}

/// Wholly owned serialization data, never an installation capability.
#[derive(Serialize)]
#[serde(transparent)]
pub struct OwnedSnapshot6(raw::Raw);

/// Schema-checked untrusted candidate with no accessible mutable authority.
pub struct ParsedSnapshot {
    authority: Authority,
}

/// Single-use admitted authority; installation always rechecks actual build state.
/// ```compile_fail,E0451
/// use sheet::snapshot6::ValidatedSnapshot;
/// let _ = ValidatedSnapshot { authority: todo!() };
/// ```
/// ```compile_fail,E0277
/// use sheet::snapshot6::ValidatedSnapshot;
/// fn requires_clone<T: Clone>() {}
/// requires_clone::<ValidatedSnapshot>();
/// ```
/// ```compile_fail,E0277
/// use sheet::snapshot6::ValidatedSnapshot;
/// fn requires_default<T: Default>() {}
/// requires_default::<ValidatedSnapshot>();
/// ```
/// ```compile_fail,E0277
/// use sheet::snapshot6::ValidatedSnapshot;
/// fn requires_decode<T: serde::de::DeserializeOwned>() {}
/// requires_decode::<ValidatedSnapshot>();
/// ```
/// ```compile_fail,E0599
/// use sheet::snapshot6::ValidatedSnapshot;
/// fn extract(value: ValidatedSnapshot) { value.into_parts(); }
/// ```
pub struct ValidatedSnapshot {
    authority: Authority,
}

/// Parse strict raw6 without consulting or mutating global state.
/// # Errors
/// Rejects missing fields, wrong types/version, duplicates, aliases or trailing input.
pub fn parse(bytes: &[u8]) -> Result<ParsedSnapshot, EvidenceError> {
    let raw = serde_json::from_slice::<raw::Raw>(bytes).map_err(|_| EvidenceError::Schema)?;
    Ok(ParsedSnapshot {
        authority: Authority::read(raw)?,
    })
}

/// Consume a parsed candidate using independently captured actual build authority.
/// # Errors
/// Rejects registry shape, configuration, provenance, cleanup or record coverage.
pub fn validate_snapshot(parsed: ParsedSnapshot) -> Result<ValidatedSnapshot, EvidenceError> {
    css::admission::with_admission(|| {
        admission::check(&parsed.authority)?;
        Ok(ValidatedSnapshot {
            authority: parsed.authority,
        })
    })
}

impl ValidatedSnapshot {
    /// Compare optional companions with the captured complete snapshot maps.
    /// # Errors
    /// Returns `Companion` for any supplied unequal map; absence is legal.
    pub fn compare_companions(
        &self,
        classes: Option<&ClassMap>,
        files: Option<&FileMap>,
    ) -> Result<(), EvidenceError> {
        if classes.is_some_and(|classes| classes != &self.authority.classes)
            || files.is_some_and(|files| files != &self.authority.files)
        {
            return Err(EvidenceError::Companion);
        }
        Ok(())
    }
}

pub(super) fn export(sheet: &StyleSheet) -> Result<OwnedSnapshot6, EvidenceError> {
    css::admission::with_admission(|| {
        sheet.counter_state.as_ref().ok_or(EvidenceError::State)?;
        let mut owned = state_live::capture_owned(sheet).map_err(EvidenceError::Kernel)?;
        let maps = state_live::maps();
        owned.source_ids = maps.originals;
        let authority = Authority {
            sheet: owned,
            classes: maps.classes,
            files: maps.files,
        };
        admission::check(&authority)?;
        let raw = authority.write();
        Authority::read(raw.clone())?;
        Ok(OwnedSnapshot6(raw))
    })
}
