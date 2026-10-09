use super::super::{authentic_support::*, state_live};
use crate::{StyleSheet, cache_snapshot::CacheRestore};

#[test]
#[serial_test::serial]
fn counter_state_stays_private_when_dense_cache4_decode_is_successful() {
    // Given
    let _state = state();
    let incoming = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &incoming).required("nonempty Counter state");
    state_live::capture_owned(&sheet).required("valid live owner");
    assert!(sheet.counter_state.is_some());
    let encoded = serde_json::to_value(sheet.export_snapshot()).required("dense cache4 export");
    assert_eq!(encoded["atomNamingVersion"], 4);
    assert_eq!(encoded.get("counter_state"), None);
    // When
    let decoded: StyleSheet = serde_json::from_value(encoded).required("dense cache4 decode");
    // Then
    assert!(
        matches!(&decoded.cache_restore, CacheRestore::Serialized { classes, files }
        if classes.values().all(|slots| slots.values().copied().eq([0]))
        && files.values().copied().eq([0]))
    );
    assert_eq!(decoded.properties, sheet.properties);
    assert_eq!(decoded.counter_state, None);
    assert!(!format!("{decoded:?}").contains("CounterState"));
}
