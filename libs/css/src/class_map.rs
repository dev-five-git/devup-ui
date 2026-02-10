use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLOBAL_CLASS_MAP: RefCell<HashMap<String, HashMap<String, usize>>> = const { RefCell::new(HashMap::new()) };
}

#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_CLASS_MAP: LazyLock<Mutex<HashMap<String, HashMap<String, usize>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline]
pub fn with_class_map<F, R>(f: F) -> R
where
    F: FnOnce(&HashMap<String, HashMap<String, usize>>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    {
        GLOBAL_CLASS_MAP.with(|map| f(&map.borrow()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        f(&GLOBAL_CLASS_MAP.lock().unwrap())
    }
}

#[inline]
pub fn with_class_map_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, HashMap<String, usize>>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    {
        GLOBAL_CLASS_MAP.with(|map| f(&mut map.borrow_mut()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        f(&mut GLOBAL_CLASS_MAP.lock().unwrap())
    }
}

/// for test
pub fn reset_class_map() {
    with_class_map_mut(|map| map.clear());
}

pub fn set_class_map(new_map: HashMap<String, HashMap<String, usize>>) {
    with_class_map_mut(|map| *map = new_map);
}

pub fn get_class_map() -> HashMap<String, HashMap<String, usize>> {
    with_class_map(|map| map.clone())
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    #[serial]
    fn test_set_and_get_class_map() {
        let mut test_map = HashMap::new();
        test_map.insert("".to_string(), HashMap::new());
        set_class_map(test_map.clone());
        let got = get_class_map();
        assert_eq!(got.get(""), Some(&HashMap::new()));
    }

    #[test]
    #[serial]
    fn test_reset_class_map() {
        let mut test_map = HashMap::new();
        test_map.insert("".to_string(), HashMap::new());
        set_class_map(test_map);
        reset_class_map();
        let got = get_class_map();
        assert!(got.is_empty());
    }
}
