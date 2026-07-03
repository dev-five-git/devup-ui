use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

// FILE_ROUTES: source filename -> set of leaf-route ids whose render closure
// includes that file. Populated by the build-time pre-pass. Used to decide,
// per atom, how many routes use it (for atom-level hoisting).
#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLOBAL_FILE_ROUTES: RefCell<HashMap<String, HashSet<u32>>> =
        RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_FILE_ROUTES: LazyLock<Mutex<HashMap<String, HashSet<u32>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[inline]
pub fn with_file_routes<F, R>(f: F) -> R
where
    F: FnOnce(&HashMap<String, HashSet<u32>>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_FILE_ROUTES.with(|map| f(&map.borrow()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = GLOBAL_FILE_ROUTES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
    }
}

#[inline]
fn with_file_routes_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, HashSet<u32>>) -> R,
{
    #[cfg(target_arch = "wasm32")]
    #[cfg(not(tarpaulin_include))]
    {
        GLOBAL_FILE_ROUTES.with(|map| f(&mut map.borrow_mut()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut guard = GLOBAL_FILE_ROUTES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }
}

/// for test
pub fn reset_file_routes() {
    with_file_routes_mut(HashMap::clear);
}

pub fn set_file_routes(new_map: HashMap<String, HashSet<u32>>) {
    with_file_routes_mut(|map| *map = new_map);
}

#[must_use]
pub fn get_file_routes() -> HashMap<String, HashSet<u32>> {
    with_file_routes(Clone::clone)
}

/// Number of DISTINCT routes across the given files (union of their route sets).
#[must_use]
pub fn route_count_for_files<'a>(files: impl IntoIterator<Item = &'a str>) -> usize {
    with_file_routes(|map| {
        // Collect the referenced route sets; the vast majority of atoms are referenced by a
        // single file, where the distinct-route count is just that file's set length and no
        // union `HashSet` allocation is needed.
        let mut sets = files.into_iter().filter_map(|file| map.get(file));
        let Some(first) = sets.next() else {
            return 0;
        };
        match sets.next() {
            None => first.len(),
            Some(second) => {
                // Pre-size the scratch set to hold at least the first two route sets so the
                // initial `extend` of `second` does not trigger a rehash-growth reallocation.
                let mut routes: HashSet<u32> = HashSet::with_capacity(first.len() + second.len());
                routes.extend(first.iter().copied());
                routes.extend(second.iter().copied());
                for set in sets {
                    routes.extend(set.iter().copied());
                }
                routes.len()
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_get_reset_file_routes() {
        let mut m = HashMap::new();
        m.insert("a.tsx".to_string(), HashSet::from([0u32, 1]));
        set_file_routes(m.clone());
        assert_eq!(get_file_routes(), m);
        reset_file_routes();
        assert!(get_file_routes().is_empty());
    }

    #[test]
    #[serial]
    fn test_route_count_for_files_union() {
        let mut m = HashMap::new();
        m.insert("a.tsx".to_string(), HashSet::from([0u32, 1]));
        m.insert("b.tsx".to_string(), HashSet::from([1u32, 2]));
        m.insert("c.tsx".to_string(), HashSet::from([5u32]));
        set_file_routes(m);
        // union of {0,1} and {1,2} = {0,1,2} -> 3
        assert_eq!(route_count_for_files(["a.tsx", "b.tsx"]), 3);
        // single file
        assert_eq!(route_count_for_files(["c.tsx"]), 1);
        // unknown file contributes nothing
        assert_eq!(route_count_for_files(["a.tsx", "zzz.tsx"]), 2);
        reset_file_routes();
    }
}
