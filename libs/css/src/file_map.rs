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
fn with_file_map<F, R>(f: F) -> R
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
        f(&GLOBAL_FILE_MAP.lock().unwrap())
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
        f(&mut GLOBAL_FILE_MAP.lock().unwrap())
    }
}

/// for test
pub fn reset_file_map() {
    with_file_map_mut(|map| map.clear());
}

pub fn set_file_map(new_map: BiHashMap<String, usize>) {
    with_file_map_mut(|map| *map = new_map);
}

pub fn get_file_map() -> BiHashMap<String, usize> {
    with_file_map(|map| map.clone())
}

#[inline]
pub fn get_file_num_by_filename(filename: &str) -> usize {
    with_file_map_mut(|map| {
        let len = map.len();
        if !map.contains_left(filename) {
            map.insert(filename.to_string(), len);
        }
        *map.get_by_left(filename).unwrap()
    })
}

pub fn get_filename_by_file_num(file_num: usize) -> String {
    with_file_map(|map| {
        map.get_by_right(&file_num)
            .map(|s| s.as_str())
            .unwrap_or_default()
            .to_string()
    })
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
}
