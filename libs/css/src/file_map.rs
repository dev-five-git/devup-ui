use bimap::BiHashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

pub(crate) static GLOBAL_FILE_MAP: Lazy<Mutex<BiHashMap<String, usize>>> =
    Lazy::new(|| Mutex::new(BiHashMap::new()));

/// for test
pub fn reset_file_map() {
    let mut map = GLOBAL_FILE_MAP.lock().unwrap();
    map.clear();
}

pub fn set_file_map(map: BiHashMap<String, usize>) {
    let mut global_map = GLOBAL_FILE_MAP.lock().unwrap();
    *global_map = map;
}

pub fn get_file_map() -> BiHashMap<String, usize> {
    GLOBAL_FILE_MAP.lock().unwrap().clone()
}

pub fn get_file_num_by_filename(filename: &str) -> usize {
    let mut map = GLOBAL_FILE_MAP.lock().unwrap();
    let len = map.len();
    if !map.contains_left(filename) {
        map.insert(filename.to_string(), len as usize);
    }
    *map.get_by_left(filename).unwrap()
}

pub fn get_filename_by_file_num(file_num: usize) -> String {
    let map = GLOBAL_FILE_MAP.lock().unwrap();
    map.get_by_right(&file_num)
        .map(|s| s.as_str())
        .unwrap_or_default()
        .to_string()
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
