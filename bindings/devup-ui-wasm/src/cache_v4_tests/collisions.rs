use super::*;

fn colored(file: &str, color: &str) -> Result<Output, String> {
    code_extract_internal(
        file,
        &format!(
            "import {{css}} from '@devup-ui/react';\nexport const x=css({{color:'{color}'}});"
        ),
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
}

#[test]
#[serial]
fn current_exact_claim_conflict_is_atomic_located_and_sticky_until_reset() {
    // Given: complete current blue proof and a live red atom from a real second site.
    reset_build_state_internal();
    colored("cached.tsx", "blue").unwrap_or_else(|error| panic!("{error}"));
    let mut value = snapshot();
    value["properties"] = serde_json::json!({});
    value["names"]["OLcolor-vred"] = value["names"]["OLcolor-vblue"].clone();
    reset_build_state_internal();
    colored("live.tsx", "red").unwrap_or_else(|error| panic!("{error}"));
    let before = snapshot();
    // When: a structurally complete current claim conflicts with live exact bytes.
    let error = import(value)
        .err()
        .unwrap_or_else(|| panic!("exact collision accepted"));
    // Then: both best sites/contents are retained, and no state is partially imported.
    for evidence in ["cached.tsx:2:27:", "live.tsx:2:27:", "`'blue'`", "`'red'`"] {
        assert!(error.contains(evidence), "{error}");
    }
    assert_eq!(snapshot(), before);
    assert_eq!(import(serde_json::json!({})), Ok(()));
    cache_restore::classes(None);
    cache_restore::files(None);
    assert_eq!(colored("later.tsx", "green").err(), Some(error.clone()));
    assert_eq!(import(before), Ok(()));
    assert_eq!(colored("later.tsx", "green").err(), Some(error));
    reset_build_state_internal();
    assert!(colored("later.tsx", "green").is_ok());
    reset_build_state_internal();
}

#[test]
#[serial]
fn corrupt_referenced_atom_descriptor_is_a_cold_miss_not_a_collision() {
    // Given: real generated output with a referenced descriptor corrupted in storage.
    reset_build_state_internal();
    let cold = compile();
    let mut value = snapshot();
    let names = value["names"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("claims"));
    let claim = names
        .values_mut()
        .find(|claim| {
            claim["descriptor"]
                .as_array()
                .is_some_and(|bytes| bytes.first() == Some(&1.into()))
        })
        .unwrap_or_else(|| panic!("atom claim"));
    claim["descriptor"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("descriptor bytes"))
        .push(99.into());
    reset_build_state_internal();
    // When: structural proof validation precedes any state mutation.
    assert_eq!(import(value), Ok(()));
    // Then: nothing is adopted and full output is independently cold.
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    assert_eq!(compile(), cold);
    reset_build_state_internal();
}

#[test]
#[serial]
fn manual_in_memory_names_remain_trusted_without_serialized_generation() {
    // Given: a freshly constructed manual sheet, not generation-less JSON.
    reset_build_state_internal();
    let mut sheet = StyleSheet::default();
    sheet.add_property(
        "manual-card",
        "color",
        0,
        "red",
        None,
        None,
        Some("card.tsx"),
    );
    // When: the existing in-memory API imports explicit construction.
    assert_eq!(import_sheet_internal(sheet), Ok(()));
    // Then: the authored manual class is still usable in emitted CSS.
    assert!(
        with_style_sheet(|sheet| sheet.create_css(Some("card.tsx"), false)).contains(concat!(
            ".manual-card{",
            "color:red",
            "}"
        ))
    );
    reset_build_state_internal();
}
