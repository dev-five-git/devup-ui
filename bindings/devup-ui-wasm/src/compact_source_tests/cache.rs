use super::*;

const SOURCE: &str =
    "import {Box} from '@devup-ui/react';\nexport const View=(p)=><Box color={p.c}/>;";

#[rstest]
#[case("'---SULmissing-b'")]
#[case("'var(---SUHaaaaaaaaaaaaaaaa-a)'")]
#[serial]
fn literal_css_content_roundtrips_when_it_resembles_a_generated_source_variable(
    #[case] content: &str,
) {
    // Given: a literal CSS string, not a generated variable consumer or reset.
    fresh();
    let literal = serde_json::to_string(content).unwrap_or_else(|error| panic!("{error}"));
    let source = format!(
        "import {{css}} from '@devup-ui/react';export const cls=css({{content:{literal}}});"
    );
    compile("/real/content.ts", &source).unwrap_or_else(|error| panic!("{error}"));
    let snapshot = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let cached = serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    fresh();
    // When: actual sheet cache import validates generated source claims.
    let restored = import_sheet_internal(cached);
    // Then: source-like text in literal content remains a valid static declaration.
    assert_eq!(restored, Ok(()));
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        snapshot
    );
    reset_build_state_internal();
}

#[rstest]
#[case("")]
#[case("Mixed---SUH-한😀-")]
#[serial]
fn source_claims_roundtrip_when_cached_sheet_is_imported_into_a_fresh_build(#[case] prefix: &str) {
    // Given: a complete cache with exact full-source claims.
    fresh();
    css::set_prefix(Some(prefix.into()));
    let before = compile("/real/a.tsx", SOURCE)
        .unwrap_or_else(|error| panic!("{error}"))
        .code();
    let css_before = with_style_sheet(|sheet| sheet.create_css(None, false));
    let snapshot = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let imported: StyleSheet =
        serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    let source_claims: BTreeMap<_, _> = imported
        .names
        .iter()
        .filter(|(name, _)| name.starts_with("---"))
        .map(|(name, claim)| (name.clone(), claim.clone()))
        .collect();
    assert_eq!(source_claims.len(), 1);
    fresh();
    css::set_prefix(Some(prefix.into()));
    // When: the cache is restored and identical text comes from another real file.
    import_sheet_internal(imported).unwrap_or_else(|error| panic!("{error}"));
    let after = compile("/real/z.tsx", SOURCE)
        .unwrap_or_else(|error| panic!("{error}"))
        .code();
    // Then: names, complete JS/CSS and exact source claims remain unchanged.
    assert_eq!(after, before);
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        css_before
    );
    with_style_sheet(|sheet| {
        for (name, claim) in source_claims {
            assert_eq!(sheet.names[&name], claim);
        }
    });
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn cache_import_fails_closed_when_only_source_claim_metadata_is_missing(
    #[case] retain_reset: bool,
) {
    // Given: all atom claims exist, but the exact source claim is missing.
    fresh();
    compile("/real/a.tsx", SOURCE).unwrap_or_else(|error| panic!("{error}"));
    let snapshot = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let mut incomplete: StyleSheet =
        serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    incomplete
        .names
        .retain(|name, _| !name.starts_with("---SUH"));
    if !retain_reset {
        for orders in incomplete.properties.values_mut() {
            for levels in orders.values_mut() {
                for properties in levels.values_mut() {
                    properties.retain(|property| !property.owner_reset);
                }
            }
        }
    }
    let rejected = import_sheet_internal(incomplete)
        .err()
        .unwrap_or_else(|| panic!("source protection is mandatory"));
    // When: a plugin catches the error and tries a subsequent extraction.
    let next = compile("/real/next.tsx", SOURCE);
    // Then: the failed import stays sticky and neither CSS nor claims is replaced.
    assert!(rejected.contains("exact source claim"), "{rejected}");
    assert_eq!(next.err(), Some(rejected));
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        snapshot
    );
    let valid = serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    import_sheet_internal(valid).unwrap_or_else(|error| panic!("{error}"));
    assert!(compile("/real/next.tsx", SOURCE).is_ok());
    reset_build_state_internal();
}

#[test]
#[serial]
fn unequal_cached_source_claims_report_both_real_files_before_replacing_the_sheet() {
    // Given: an existing exact source claim and a different full source under its name.
    fresh();
    compile("/real/first.tsx", SOURCE).unwrap_or_else(|error| panic!("{error}"));
    let snapshot = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let cached: StyleSheet =
        serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    let (name, mut claim) = cached
        .names
        .into_iter()
        .find(|(name, _)| name.starts_with("---SUH"))
        .unwrap_or_else(|| panic!("source claim"));
    let other = format!("{SOURCE}\n// another received source");
    claim.content.clone_from(&other);
    claim.descriptor = other.as_bytes().to_vec();
    let css::style_origin::RealLocation::Exact(origin) = &mut claim.location else {
        panic!("real witness")
    };
    origin.file = "/real/second.tsx".into();
    claim.origin = Some(origin.clone());
    let mut incoming = StyleSheet::default();
    incoming.names.insert(name, claim);
    // When: importing overlapping claims without any atom descriptor difference.
    let error = import_sheet_internal(incoming)
        .err()
        .unwrap_or_else(|| panic!("unequal exact source"));
    // Then: both real witnesses/full sources are printed and the old sheet survives.
    assert!(error.contains("/real/first.tsx:2:"), "{error}");
    assert!(error.contains("/real/second.tsx:2:"), "{error}");
    assert!(error.contains(SOURCE), "{error}");
    assert!(error.contains(&other), "{error}");
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        snapshot
    );
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn cache_import_rejects_corrupted_source_proof_even_when_the_named_entry_exists(
    #[case] corrupt_content: bool,
) {
    // Given: a source variable whose claim exists but has inconsistent proof bytes.
    fresh();
    compile("/real/a.tsx", SOURCE).unwrap_or_else(|error| panic!("{error}"));
    let snapshot = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let mut corrupt: StyleSheet =
        serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    let claim = corrupt
        .names
        .iter_mut()
        .find(|(name, _)| name.starts_with("---SUH"))
        .unwrap_or_else(|| panic!("source claim"))
        .1;
    if corrupt_content {
        claim.content.push_str("\n// forged source");
        claim.descriptor = claim.content.as_bytes().to_vec();
    } else {
        claim.descriptor.push(0);
    }
    // When: the cache boundary checks exact proof and content-only naming together.
    let error = import_sheet_internal(corrupt)
        .err()
        .unwrap_or_else(|| panic!("invalid source proof"));
    // Then: existence alone cannot bypass collision protection.
    assert!(error.contains("invalid exact source claim"), "{error}");
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        snapshot
    );
    reset_build_state_internal();
}
