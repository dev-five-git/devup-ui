use std::{
    collections::BTreeMap,
    sync::{LazyLock, Mutex},
};

static ORIGINAL_IDS: LazyLock<Mutex<BTreeMap<String, u32>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

fn with_original_ids<R>(f: impl FnOnce(&mut BTreeMap<String, u32>) -> R) -> R {
    f(&mut ORIGINAL_IDS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// D9 number of `file`, if it belongs to the numbered original sources.
#[must_use]
pub fn original_id(file: &str) -> Option<u32> {
    with_original_ids(|ids| ids.get(file).copied())
}

#[must_use]
pub fn get_original_ids() -> BTreeMap<String, u32> {
    with_original_ids(|ids| ids.clone())
}

pub fn set_original_ids(new_ids: BTreeMap<String, u32>) {
    with_original_ids(|ids| *ids = new_ids);
}

pub(crate) fn seed_original_ids(files: &[String]) {
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
