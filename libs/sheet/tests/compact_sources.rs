use css::{content_hash::FingerprintBits, content_name::AtomContent, style_origin::RealLocation};
use extractor::{
    ExtractOption, extract_style::extract_style_value::ExtractStyleValue,
    extract_without_source_map,
};
use serial_test::serial;
use sheet::{
    StyleSheet,
    name_registry::{NameClaim, NameRegistry, preflight},
};

#[test]
#[serial]
fn different_sources_are_rejected_when_narrow_variables_and_atom_descriptors_match() {
    // Given: three real files with the same dynamic site competing for one bit.
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::set_prefix(None);
    let bits = FingerprintBits::new(1).unwrap_or_else(|| panic!("valid width"));
    let records: Vec<_> = (0..3).map(|index| {
        let file = format!("/real/source{index}.tsx");
        let source = format!("// {index}\nimport {{Box}} from '@devup-ui/react';\nexport const View=(p)=><Box color={{p.c}}/>;");
        let output = extract_without_source_map(&file, &source, ExtractOption::default()).unwrap_or_else(|error| panic!("{error}"));
        let claims = StyleSheet::default().preflight_styles_with_bits(&output.styles, (&file, true), bits).unwrap_or_else(|error| panic!("{error}"));
        let (name, claim) = claims.into_iter().find(|(name, _)| name.starts_with("---SUH")).unwrap_or_else(|| panic!("source claim"));
        let variable = output.styles.iter().find_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => style.site().map(|site| site.variable_name_with_bits("", bits)),
            ExtractStyleValue::Static(_) | ExtractStyleValue::Typography(_) | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_) | ExtractStyleValue::FontFace(_) | ExtractStyleValue::Keyframes(_) => None,
        }).unwrap_or_else(|| panic!("dynamic site"));
        (file, source, output.styles, name, claim, variable)
    }).collect();
    let (first, second) = (0..3)
        .flat_map(|first| ((first + 1)..3).map(move |second| (first, second)))
        .find(|(first, second)| records[*first].3 == records[*second].3)
        .unwrap_or_else(|| panic!("pigeonhole collision"));
    let left = &records[first];
    let right = &records[second];
    assert_eq!(left.5, right.5);
    let atom = |variable: &str| {
        AtomContent {
            property: "color",
            level: 0,
            value: Some(variable),
            selector: None,
            order: 255,
            naming: css::Naming::Own,
            layer: None,
            dynamic: true,
        }
        .content()
    };
    let left_atom = atom(&left.5);
    let right_atom = atom(&right.5);
    assert_eq!(left_atom.descriptor, right_atom.descriptor);
    let mut sheet = StyleSheet::default();
    sheet.names.insert(left.3.clone(), left.4.clone());
    let serialized =
        serde_json::to_string(&sheet.export_snapshot()).unwrap_or_else(|error| panic!("{error}"));
    let restored: StyleSheet =
        serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}"));
    // When: the actual production preflight sees another source, not another CSS atom.
    let error = restored
        .preflight_styles_with_bits(&right.2, (&right.0, true), bits)
        .err()
        .unwrap_or_else(|| panic!("exact source collision"));
    // Then: both full sources and both authored witnesses survive the cache roundtrip.
    assert_eq!(error.name, left.3);
    assert_ne!(error.first.descriptor, error.second.descriptor);
    assert_eq!(restored.names, sheet.names);
    for record in [left, right] {
        let claim = [&error.first, &error.second]
            .into_iter()
            .find(|claim| claim.content == record.1)
            .unwrap_or_else(|| panic!("full source claim"));
        let RealLocation::Exact(origin) = &claim.location else {
            panic!("expected real authored site")
        };
        assert_eq!(origin.file, record.0);
        assert_eq!(origin.line, 3);
        let offset = record
            .1
            .find(&origin.expression)
            .unwrap_or_else(|| panic!("authored expression"));
        let column = record.1[..offset]
            .rsplit('\n')
            .next()
            .unwrap_or_else(|| panic!("source line"))
            .chars()
            .count()
            + 1;
        assert_eq!(origin.column, column);
        assert!(error.to_string().contains(&claim.content));
        assert!(error.to_string().contains(&format!(
            "{}:{}:{}:",
            origin.file, origin.line, origin.column
        )));
    }
    assert!(error.to_string().contains("rebuild all caches"));

    let atom_name = left_atom.name_with_bits("", bits);
    let shared_atom = NameClaim {
        descriptor: left_atom.descriptor,
        ..left.4.clone()
    };
    let registry = NameRegistry::from([
        (left.3.clone(), left.4.clone()),
        (atom_name.clone(), shared_atom.clone()),
    ]);
    assert!(
        preflight(
            &registry,
            [(atom_name, shared_atom), (right.3.clone(), right.4.clone())]
        )
        .is_err()
    );
}

#[test]
#[serial]
fn source_claims_ignore_counter_scope_and_keep_best_actual_witness() {
    // Given: identical source text in differently named files and canonical buckets.
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::set_prefix(None);
    let source = "import {Box} from '@devup-ui/react';\nexport const View=(p)=><Box color={p.c}/>;";
    let mut sheet = StyleSheet::default();
    // When: both real files contribute the same source to the shared registry.
    for file in ["/real/z.tsx", "/real/a.tsx"] {
        let output = extract_without_source_map(file, source, ExtractOption::default())
            .unwrap_or_else(|error| panic!("{error}"));
        let claims = sheet
            .preflight_styles(&output.styles, file, false)
            .unwrap_or_else(|error| panic!("{error}"));
        sheet.names.extend(claims);
    }
    // Then: the source has one content-only claim with the best real location.
    let sources: Vec<_> = sheet
        .names
        .iter()
        .filter(|(name, _)| name.starts_with("---SUH"))
        .collect();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].1.content, source);
    assert!(
        matches!(&sources[0].1.location, RealLocation::Exact(origin) if origin.file == "/real/a.tsx")
    );
}
