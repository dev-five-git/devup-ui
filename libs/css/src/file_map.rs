use bimap::BiHashMap;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLOBAL_FILE_MAP: RefCell<BiHashMap<String, usize>> = RefCell::new(BiHashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_FILE_MAP: LazyLock<Mutex<BiHashMap<String, usize>>> =
    LazyLock::new(|| Mutex::new(BiHashMap::new()));

#[inline]
pub fn with_file_map<F, R>(f: F) -> R
where
    F: FnOnce(&BiHashMap<String, usize>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_FILE_MAP.with(|map| f(&map.borrow()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = GLOBAL_FILE_MAP
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
    }
}

#[inline]
fn with_file_map_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut BiHashMap<String, usize>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_FILE_MAP.with(|map| f(&mut map.borrow_mut()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut guard = GLOBAL_FILE_MAP
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }
}

/// for test
pub fn reset_file_map() {
    with_file_map_mut(BiHashMap::clear);
}

pub fn set_file_map(new_map: BiHashMap<String, usize>) {
    with_file_map_mut(|map| *map = new_map);
}

pub fn get_file_map() -> BiHashMap<String, usize> {
    with_file_map(Clone::clone)
}

#[inline]
#[must_use]
pub fn get_file_num_by_filename(filename: &str) -> usize {
    with_file_map_mut(|map| {
        if let Some(&file_num) = map.get_by_left(filename) {
            file_num
        } else {
            let file_num = map.len();
            map.insert(filename.to_string(), file_num);
            file_num
        }
    })
}

#[must_use]
pub fn get_filename_by_file_num(file_num: usize) -> String {
    with_file_map(|map| {
        map.get_by_right(&file_num)
            .map(String::as_str)
            .unwrap_or_default()
            .to_string()
    })
}

// CANONICAL_MAP: real filename -> bucket-root filename. Populated by a build-time
// pre-pass (single-importer collapse). When empty, `canonical()` is the identity
// so existing behavior (and all snapshots) is unchanged — the dedup is opt-in.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLOBAL_CANONICAL_MAP: RefCell<std::collections::HashMap<String, String>> =
        RefCell::new(std::collections::HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_CANONICAL_MAP: LazyLock<Mutex<std::collections::HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

#[inline]
pub fn with_canonical_map<F, R>(f: F) -> R
where
    F: FnOnce(&std::collections::HashMap<String, String>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_CANONICAL_MAP.with(|map| f(&map.borrow()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = GLOBAL_CANONICAL_MAP
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
    }
}

#[inline]
fn with_canonical_map_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut std::collections::HashMap<String, String>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_CANONICAL_MAP.with(|map| f(&mut map.borrow_mut()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut guard = GLOBAL_CANONICAL_MAP
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }
}

/// for test
pub fn reset_canonical_map() {
    with_canonical_map_mut(std::collections::HashMap::clear);
}

pub fn set_canonical_map(new_map: std::collections::HashMap<String, String>) {
    with_canonical_map_mut(|map| *map = new_map);
}

#[must_use]
pub fn get_canonical_map() -> std::collections::HashMap<String, String> {
    with_canonical_map(Clone::clone)
}

/// Resolve a filename to its bucket-root via `CANONICAL_MAP`, or identity when absent.
#[must_use]
pub fn canonical(filename: &str) -> String {
    with_canonical_map(|map| {
        map.get(filename)
            .map_or_else(|| filename.to_string(), Clone::clone)
    })
}

/// Sentinel `CANONICAL_MAP` value marking a file for global emission.
///
/// Such a file's styles are hoisted into the global `devup-ui.css` (shared chunk)
/// with single-css naming, so styles shared across many routes ship once. Set by
/// the route-reachability pre-pass.
pub const GLOBAL_BUCKET: &str = "@global";

/// Whether a file is marked for global (shared-chunk) emission.
#[must_use]
pub fn is_global(filename: &str) -> bool {
    with_canonical_map(|map| map.get(filename).map(String::as_str) == Some(GLOBAL_BUCKET))
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_set_and_get_file_map() {
        let mut test_map = BiHashMap::new();
        test_map.insert("test-key".to_string(), 42);
        set_file_map(test_map.clone());
        let got = get_file_map();
        assert_eq!(got.get_by_left("test-key"), Some(&42));
        assert_eq!(got.get_by_right(&42), Some(&"test-key".to_string()));
        assert_eq!(get_file_num_by_filename("test-key"), 42);
        assert_eq!(get_filename_by_file_num(42), "test-key");
    }

    #[test]
    #[serial]
    fn test_reset_file_map() {
        let mut test_map = BiHashMap::new();
        test_map.insert("reset-key".to_string(), 1);
        set_file_map(test_map);
        reset_file_map();
        let got = get_file_map();
        assert!(got.is_empty());
    }

    #[test]
    #[serial]
    fn test_canonical_identity_when_empty() {
        reset_canonical_map();
        assert_eq!(canonical("a.tsx"), "a.tsx");
    }

    #[test]
    #[serial]
    fn test_canonical_mapped_and_unmapped() {
        let mut m = std::collections::HashMap::new();
        m.insert("child.tsx".to_string(), "parent.tsx".to_string());
        set_canonical_map(m);
        // mapped -> bucket root
        assert_eq!(canonical("child.tsx"), "parent.tsx");
        // unmapped -> identity
        assert_eq!(canonical("other.tsx"), "other.tsx");
        reset_canonical_map();
    }

    #[test]
    #[serial]
    fn test_canonical_map_roundtrip() {
        let mut m = std::collections::HashMap::new();
        m.insert("a".to_string(), "b".to_string());
        set_canonical_map(m.clone());
        assert_eq!(get_canonical_map(), m);
        reset_canonical_map();
        assert!(get_canonical_map().is_empty());
    }

    #[test]
    #[serial]
    fn test_is_global() {
        reset_canonical_map();
        assert!(!is_global("shared.tsx"));
        let mut m = std::collections::HashMap::new();
        m.insert("shared.tsx".to_string(), GLOBAL_BUCKET.to_string());
        m.insert("child.tsx".to_string(), "parent.tsx".to_string());
        set_canonical_map(m);
        assert!(is_global("shared.tsx")); // sentinel => global
        assert!(!is_global("child.tsx")); // normal collapse => not global
        assert!(!is_global("absent.tsx")); // absent => not global
        reset_canonical_map();
    }
}
