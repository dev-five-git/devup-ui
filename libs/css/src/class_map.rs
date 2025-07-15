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
