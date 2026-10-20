use super::*;
use std::collections::{HashMap, HashSet};

#[test]
#[serial]
fn restored_legacy_rule_stays_local_when_new_rule_is_hoisted_in_same_bucket() {
    // Given
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::file_routes::reset_file_routes();
    css::file_map::reset_canonical_map();
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    css::set_prefix(None);
    css::debug::set_debug(false);
    let mut legacy = StyleSheet::default();
    legacy.set_theme(Theme::default());
    legacy.add_property("legacy", "color", 0, "red", None, None, Some("mixed.tsx"));
    let persisted = serde_json::to_string(&legacy).unwrap_or_else(|error| panic!("{error}"));
    assert!(!persisted.contains("\"h\":"), "{persisted}");
    let mut sheet: StyleSheet =
        serde_json::from_str(&persisted).unwrap_or_else(|error| panic!("{error}"));
    css::file_routes::set_file_routes(HashMap::from([(
        "mixed.tsx".to_string(),
        HashSet::from([0, 1]),
    )]));
    css::atom_hoist::set_atom_hoist(Some(2));

    // When
    let output = extractor::extract(
        "mixed.tsx",
        "import {Box} from '@devup-ui/react'; export const x=<Box bg=\"blue\"/>;",
        extractor::ExtractOption::default(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    sheet.update_styles(&output.styles, "mixed.tsx", false);
    let shared = sheet.create_css(None, false);
    let local = sheet.create_css(Some("mixed.tsx"), false);

    // Then
    let class_name = output
        .code
        .split("className=\"")
        .nth(1)
        .and_then(|code| code.split('"').next())
        .unwrap_or_else(|| panic!("{}", output.code));
    assert_eq!(class_name.split_whitespace().count(), 1, "{}", output.code);
    assert_ne!(class_name, "legacy");
    let new_rule = format!(".{class_name}{{background:blue}}");
    assert_eq!(shared.matches(&new_rule).count(), 1, "{shared}");
    assert!(!shared.contains(".legacy"), "{shared}");
    assert_eq!(
        local,
        format!("{}.legacy{{color:red}}", StyleSheet::create_header())
    );
    assert!(!local.contains(&format!(".{class_name}")), "{local}");
    let bucket = &sheet.properties["mixed.tsx"][&255][&0];
    let dispositions: BTreeSet<_> = bucket
        .iter()
        .map(|prop| (prop.class_name.as_str(), prop.hoisted))
        .collect();
    assert_eq!(
        dispositions,
        BTreeSet::from([("legacy", false), (class_name, true)])
    );

    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::file_routes::reset_file_routes();
    css::file_map::reset_canonical_map();
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    css::set_prefix(None);
    css::debug::set_debug(false);
    StyleSheet::default().set_theme(Theme::default());
}
