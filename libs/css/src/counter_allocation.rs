use std::collections::HashMap;

use crate::{class_map, num_to_nm_base::num_to_nm_base};

/// A numeric slot obtained from a class-map namespace, not a content or debug name.
///
/// Slots are namespace-local and remain subject to class-map attempt rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CounterSlot(usize);

impl CounterSlot {
    /// Return the actual numeric slot from the map lookup or insertion.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }

    /// Render the existing base-37 spelling, including the ad-blocker splice.
    #[must_use]
    pub fn name(self) -> String {
        num_to_nm_base(self.0)
    }
}

/// Borrow-probe an exact key, allocating owned key bytes only on insertion.
pub(crate) fn reserve_counter(namespace: &str, key: &str) -> CounterSlot {
    class_map::with_class_map_mut(|map| {
        if let Some(file_entry) = map.get_mut(namespace) {
            if let Some(&num) = file_entry.get(key) {
                CounterSlot(num)
            } else {
                let len = file_entry.len();
                file_entry.insert(key.to_string(), len);
                class_map::record_insert(namespace, key);
                CounterSlot(len)
            }
        } else {
            let mut inner = HashMap::with_capacity(1);
            inner.insert(key.to_string(), 0);
            map.insert(namespace.to_string(), inner);
            class_map::record_insert(namespace, key);
            CounterSlot(0)
        }
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serial_test::serial;

    use super::*;
    use crate::class_map::{Attempt, get_class_map, reset_class_map, set_class_map};

    #[test]
    #[serial]
    fn reuses_slot_when_key_repeats() {
        // Given: a key already allocated in the real map.
        reset_class_map();
        let first = reserve_counter("D9-0", " exact-키 ");
        let before = get_class_map();
        // When: the exact key is requested again.
        let repeated = reserve_counter("D9-0", " exact-키 ");
        // Then: neither the slot nor the map changes.
        assert_eq!(repeated, first);
        assert_eq!(get_class_map(), before);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn allocates_next_slot_when_key_is_distinct() {
        // Given: one allocation, with whitespace significant to key identity.
        reset_class_map();
        reserve_counter("D9-0", "key");
        // When: a distinct key is requested in the existing namespace.
        let next = reserve_counter("D9-0", " key ");
        // Then: the slot follows the existing map length without normalization.
        assert_eq!((next.index(), next.name()), (1, "b".to_string()));
        assert_eq!(
            get_class_map(),
            HashMap::from([(
                "D9-0".to_string(),
                HashMap::from([("key".to_string(), 0), (" key ".to_string(), 1)]),
            )])
        );
        reset_class_map();
    }

    #[test]
    #[serial]
    fn starts_at_zero_when_namespace_is_distinct() {
        // Given: a private namespace with two keys.
        reset_class_map();
        reserve_counter("D9-0", "key");
        reserve_counter("D9-0", "other");
        // When: the same key is requested in the shared namespace.
        let shared = reserve_counter("", "other");
        // Then: the namespaces retain independent streams.
        assert_eq!((shared.index(), shared.name()), (0, "a".to_string()));
        assert_eq!(get_class_map()["D9-0"]["other"], 1);
        assert_eq!(get_class_map()[""]["other"], 0);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn reuses_stored_slot_when_map_is_preseeded() {
        // Given: an existing slot whose value differs from the map length.
        let seeded = HashMap::from([(
            "D9-7".to_string(),
            HashMap::from([("seeded".to_string(), 30)]),
        )]);
        set_class_map(seeded.clone());
        // When: the seeded key is reused during an abandoned attempt.
        let reused = {
            let _attempt = Attempt::begin();
            reserve_counter("D9-7", "seeded")
        };
        // Then: lookup returns the stored slot and rollback leaves it intact.
        assert_eq!((reused.index(), reused.name()), (30, "a-d".to_string()));
        assert_eq!(get_class_map(), seeded);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn releases_insertions_when_attempt_is_abandoned() {
        // Given: one retained entry and one as-yet unseen namespace.
        reset_class_map();
        reserve_counter("D9-0", "retained");
        // When: both insertion paths run inside an abandoned attempt.
        {
            let _attempt = Attempt::begin();
            reserve_counter("D9-0", "discarded");
            reserve_counter("D9-1", "discarded");
        }
        // Then: only inserted keys are removed; their slots are available again.
        assert_eq!(
            get_class_map(),
            HashMap::from([
                (
                    "D9-0".to_string(),
                    HashMap::from([("retained".to_string(), 0)])
                ),
                ("D9-1".to_string(), HashMap::new()),
            ])
        );
        assert_eq!(reserve_counter("D9-0", "replacement").index(), 1);
        assert_eq!(reserve_counter("D9-1", "replacement").index(), 0);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn rolls_back_committed_inner_when_outer_is_abandoned() {
        // Given: a retained entry outside the outer attempt.
        reset_class_map();
        reserve_counter("D9-0", "retained");
        // When: the inner attempt commits both insertion paths, but the outer drops.
        {
            let _outer = Attempt::begin();
            let inner = Attempt::begin();
            reserve_counter("D9-0", "inner");
            reserve_counter("D9-1", "inner");
            inner.commit();
        }
        // Then: the inner commit did not escape the outer rollback journal.
        assert_eq!(
            get_class_map(),
            HashMap::from([
                (
                    "D9-0".to_string(),
                    HashMap::from([("retained".to_string(), 0)])
                ),
                ("D9-1".to_string(), HashMap::new()),
            ])
        );
        reset_class_map();
    }

    #[rstest]
    #[case(26, "_")]
    #[case(27, "aa")]
    #[case(29, "ac")]
    #[case(30, "a-d")]
    #[case(31, "ae")]
    #[serial]
    fn renders_boundary_when_slot_is_inserted(#[case] count: usize, #[case] expected: &str) {
        // Given: a dense namespace with exactly count existing slots.
        set_class_map(HashMap::from([(
            "D9-30".to_string(),
            (0..count)
                .map(|index| (format!("seed-{index}"), index))
                .collect(),
        )]));
        // When: the next slot is actually allocated, not constructed from spelling.
        let slot = reserve_counter("D9-30", "boundary");
        // Then: the numeric address and existing base-37 spelling agree.
        assert_eq!(slot.index(), count);
        assert_eq!(slot.name(), expected);
        assert_eq!(get_class_map()["D9-30"]["boundary"], count);
        reset_class_map();
    }

    #[test]
    #[serial]
    fn preserves_file_and_slot_splices_when_owned_output_crosses_ad() {
        // Given: original file 30 and thirty already allocated keys.
        set_class_map(HashMap::from([(
            "D9-30".to_string(),
            (0..30)
                .map(|index| (format!("seed-{index}"), index))
                .collect(),
        )]));
        let prefix = crate::get_prefix();
        crate::set_prefix(None);
        let content = crate::content_name::AtomContent {
            property: "color",
            value: Some("red"),
            level: 0,
            order: 255,
            naming: crate::Naming::Own,
            selector: None,
            layer: None,
            dynamic: false,
        };
        // When: the existing owned caller allocates and renders slot 30.
        let name = crate::sheet_to_classname_owned(
            &content,
            Some("counter-boundary.tsx"),
            crate::CounterOwner::D9(30),
        );
        // Then: both file and slot retain their separate ad-blocker splices.
        assert_eq!(name, "a-d-a-d");
        crate::set_prefix(prefix);
        reset_class_map();
    }
}
