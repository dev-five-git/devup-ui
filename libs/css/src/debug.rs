use std::sync::Mutex;

static DEBUG: Mutex<bool> = Mutex::new(false);

pub fn set_debug(value: bool) {
    let mut debug = DEBUG.lock().unwrap();
    *debug = value;
}

pub fn is_debug() -> bool {
    *DEBUG.lock().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_debug() {
        set_debug(true);
        assert!(is_debug());
        set_debug(false);
        assert!(!is_debug());
    }
}
