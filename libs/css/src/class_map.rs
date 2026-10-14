use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLOBAL_CLASS_MAP: RefCell<HashMap<String, HashMap<String, usize>>> = RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_CLASS_MAP: LazyLock<Mutex<HashMap<String, HashMap<String, usize>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline]
pub fn with_class_map<F, R>(f: F) -> R
where
    F: FnOnce(&HashMap<String, HashMap<String, usize>>) -> R,
{
    let _admission = crate::admission::enter();
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_CLASS_MAP.with(|map| f(&map.borrow()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _root = crate::root_held::RootHeld::enter("class_map");
        let guard = GLOBAL_CLASS_MAP
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
    }
}

#[inline]
pub fn with_class_map_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, HashMap<String, usize>>) -> R,
{
    let _admission = crate::admission::enter();
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_CLASS_MAP.with(|map| f(&mut map.borrow_mut()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _root = crate::root_held::RootHeld::enter("class_map");
        let mut guard = GLOBAL_CLASS_MAP
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }
}

/// for test
pub fn reset_class_map() {
    with_class_map_mut(HashMap::clear);
}

pub fn set_class_map(new_map: HashMap<String, HashMap<String, usize>>) {
    with_class_map_mut(|map| *map = new_map);
}

pub fn get_class_map() -> HashMap<String, HashMap<String, usize>> {
    with_class_map(Clone::clone)
}

#[derive(Default)]
struct Journal {
    attempts: usize,
    inserted: Vec<(String, String)>,
}

thread_local! {
    static JOURNAL: std::cell::RefCell<Journal> = std::cell::RefCell::new(Journal::default());
}

/// Remember a name handed out while an [`Attempt`] runs, so dropping the
/// attempt gives the slot back.
pub(crate) fn record_insert(filename_key: &str, key: &str) {
    let _admission = crate::admission::enter();
    JOURNAL.with(|journal| {
        let _root = crate::root_held::RootHeld::enter("journal");
        let mut journal = journal.borrow_mut();
        if journal.attempts > 0 {
            journal
                .inserted
                .push((filename_key.to_string(), key.to_string()));
        }
    });
}

/// A stretch of extraction whose names are kept only when it is committed.
///
/// Names an attempt hands out that is then discarded (an evaluation retry or a
/// build error) would otherwise leave counter slots that a later environment
/// finds taken.
pub struct Attempt {
    mark: usize,
    committed: bool,
}

impl Attempt {
    #[must_use]
    pub fn begin() -> Self {
        let _admission = crate::admission::enter();
        JOURNAL.with(|journal| {
            let _root = crate::root_held::RootHeld::enter("journal");
            let mut journal = journal.borrow_mut();
            journal.attempts += 1;
            Self {
                mark: journal.inserted.len(),
                committed: false,
            }
        })
    }

    pub fn commit(mut self) {
        let _admission = crate::admission::enter();
        self.committed = true;
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        let _admission = crate::admission::enter();
        let undone = JOURNAL.with(|journal| {
            let _root = crate::root_held::RootHeld::enter("journal");
            let mut journal = journal.borrow_mut();
            journal.attempts -= 1;
            let undone = if self.committed {
                Vec::new()
            } else {
                journal.inserted.split_off(self.mark)
            };
            if journal.attempts == 0 {
                journal.inserted.clear();
            }
            undone
        });
        if !undone.is_empty() {
            with_class_map_mut(|map| {
                for (file, key) in undone.iter().rev() {
                    if let Some(names) = map.get_mut(file) {
                        names.remove(key);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_set_and_get_class_map() {
        let mut test_map = HashMap::new();
        test_map.insert(String::new(), HashMap::new());
        set_class_map(test_map.clone());
        let got = get_class_map();
        assert_eq!(got.get(""), Some(&HashMap::new()));
    }

    #[test]
    #[serial]
    fn test_reset_class_map() {
        let mut test_map = HashMap::new();
        test_map.insert(String::new(), HashMap::new());
        set_class_map(test_map);
        reset_class_map();
        let got = get_class_map();
        assert!(got.is_empty());
    }
}
