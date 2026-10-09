use std::{
    collections::BTreeMap,
    num::TryFromIntError,
    sync::{LazyLock, Mutex},
};

static ORIGINAL_IDS: LazyLock<Mutex<BTreeMap<String, u32>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

fn with_original_ids<R>(f: impl FnOnce(&mut BTreeMap<String, u32>) -> R) -> R {
    let _admission = crate::admission::enter();
    let _root = crate::root_held::RootHeld::enter("original_ids");
    f(&mut ORIGINAL_IDS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// D9 number of `file`, if it belongs to the numbered original sources.
#[must_use]
pub fn original_id(file: &str) -> Option<u32> {
    with_original_ids(|ids| ids.get(file).copied())
}

fn original_id_from_len(len: usize) -> Result<u32, TryFromIntError> {
    u32::try_from(len)
}

/// Reuse the exact original filename's D9 number, or register it at the current length.
///
/// # Errors
/// Returns `TryFromIntError` without inserting when the registry length exceeds `u32::MAX`.
pub fn get_or_insert_original_id(filename: &str) -> Result<u32, TryFromIntError> {
    with_original_ids(|ids| {
        if let Some(id) = ids.get(filename) {
            return Ok(*id);
        }
        let id = original_id_from_len(ids.len())?;
        ids.insert(filename.to_string(), id);
        Ok(id)
    })
}

#[must_use]
pub fn get_original_ids() -> BTreeMap<String, u32> {
    with_original_ids(|ids| ids.clone())
}

pub fn set_original_ids(new_ids: BTreeMap<String, u32>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_original_ids");
    with_original_ids(|ids| *ids = new_ids);
}

/// Restore only the original-ID snapshot owned by a private exact attempt.
pub(crate) fn restore_original_ids_snapshot(snapshot: BTreeMap<String, u32>) {
    with_original_ids(|ids| *ids = snapshot);
}

pub(crate) fn seed_original_ids(files: &[String]) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("seed_original_ids");
    let mut originals: Vec<&String> = files.iter().collect();
    originals.sort_unstable();
    originals.dedup();
    with_original_ids(|ids| {
        for file in originals {
            if !ids.contains_key(file)
                && let Ok(id) = u32::try_from(ids.len())
            {
                ids.insert(file.clone(), id);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn registration_preserves_set_ids_when_their_numbers_differ_from_length() {
        // Given: restored IDs are not inferred from their positions.
        set_original_ids(BTreeMap::from([("seed.tsx".into(), 42)]));
        // When: an existing filename and a new exact filename are registered.
        let reused = get_or_insert_original_id("seed.tsx");
        let inserted = get_or_insert_original_id("Seed.tsx");
        // Then: set IDs survive and append uses length, not max ID or path normalization.
        assert_eq!(reused, Ok(42));
        assert_eq!(inserted, Ok(1));
        assert_eq!(original_id("Seed.tsx"), Some(1));
        set_original_ids(BTreeMap::new());
    }

    #[test]
    fn original_id_conversion_accepts_zero_when_registry_is_empty() {
        // Given / When: the empty registry's length is converted.
        let id = original_id_from_len(0);
        // Then: the first original gets zero.
        assert_eq!(id, Ok(0));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn original_id_conversion_rejects_length_when_it_exceeds_u32() {
        // Given: the last representable ID, without allocating registry entries.
        let last = usize::try_from(u32::MAX).unwrap_or_else(|error| panic!("{error}"));
        // When: the boundary and its successor are converted by the production helper.
        let boundary = original_id_from_len(last);
        let overflow = original_id_from_len(last + 1);
        // Then: overflow is a typed conversion error, never an aliased ID.
        assert_eq!(boundary, Ok(u32::MAX));
        assert!(overflow.is_err());
    }
}
