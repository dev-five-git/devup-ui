use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;

pub(crate) static GLOBAL_CLASS_MAP: Lazy<Mutex<HashMap<String, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// for test
pub fn reset_class_map() {
    let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
    map.clear();
}

pub fn set_class_map(map: HashMap<String, i32>) {
    let mut global_map = GLOBAL_CLASS_MAP.lock().unwrap();
    *global_map = map;
}

pub fn get_class_map() -> HashMap<String, i32> {
    GLOBAL_CLASS_MAP.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_set_and_get_class_map() {
        let mut test_map = HashMap::new();
        test_map.insert("test-key".to_string(), 42);
        set_class_map(test_map.clone());
        let got = get_class_map();
        assert_eq!(got.get("test-key"), Some(&42));
    }

    #[test]
    #[serial]
    fn test_reset_class_map() {
        let mut test_map = HashMap::new();
        test_map.insert("reset-key".to_string(), 1);
        set_class_map(test_map);
        reset_class_map();
        let got = get_class_map();
        assert!(got.is_empty());
    }
}
