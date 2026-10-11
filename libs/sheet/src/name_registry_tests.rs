use super::{
    StyleSheet,
    name_registry::{NameClaim, preflight},
};
use css::style_origin::{RealLocation, StyleOrigin};
use std::collections::BTreeMap;

fn claim(file: &str, descriptor: &[u8]) -> NameClaim {
    let origin = StyleOrigin {
        file: file.into(),
        line: 7,
        column: 9,
        expression: "style({color: token})".into(),
    };
    NameClaim {
        descriptor: descriptor.to_vec(),
        content: String::from_utf8_lossy(descriptor).into_owned(),
        origin: Some(origin.clone()),
        location: RealLocation::Exact(origin),
    }
}

#[test]
fn unequal_claims_report_both_actual_sites_without_changing_registry() {
    // Given
    let names = BTreeMap::from([("OHforced".into(), claim("a.css.ts", b"red"))]);
    let before = names.clone();
    // When
    let Err(error) = preflight(&names, [("OHforced".into(), claim("b.css.ts", b"blue"))]) else {
        panic!("expected unequal-claim collision")
    };
    // Then
    assert_eq!(*error.first, before["OHforced"]);
    assert_eq!(*error.second, claim("b.css.ts", b"blue"));
    assert_eq!(names, before);
    let text = error.to_string();
    assert!(text.contains("a.css.ts:7:9:"), "{text}");
    assert!(text.contains("b.css.ts:7:9:"), "{text}");
    assert!(text.contains("OHforced"), "{text}");
}

#[test]
fn batch_collisions_are_rejected_before_any_claim_is_registered() {
    // Given
    let names = BTreeMap::new();
    // When
    let result = preflight(
        &names,
        [
            ("RHsame".into(), claim("a.tsx", b"one")),
            ("RHsame".into(), claim("b.tsx", b"two")),
        ],
    );
    // Then
    assert!(result.is_err());
    assert_eq!(names.len(), 0);
}

#[test]
fn equal_claims_keep_the_minimum_actual_origin_across_cache_roundtrips() {
    // Given
    let mut sheet = StyleSheet::default();
    sheet
        .names
        .insert("RHsame".into(), claim("z.css.ts", b"same"));
    let serialized = serde_json::to_string(&sheet).unwrap_or_else(|error| panic!("{error}"));
    let mut restored: StyleSheet =
        serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}"));
    // When
    let batch = preflight(
        &restored.names,
        [("RHsame".into(), claim("a.css.ts", b"same"))],
    )
    .unwrap_or_else(|error| panic!("{error}"));
    restored.names.extend(batch);
    // Then
    assert_eq!(restored.names["RHsame"], claim("a.css.ts", b"same"));
    assert!(
        preflight(
            &restored.names,
            [("RHsame".into(), claim("c.css.ts", b"different"))]
        )
        .is_err()
    );
}
