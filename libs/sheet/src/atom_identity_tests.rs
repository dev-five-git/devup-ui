use super::*;
use css::sheet_to_classname;
use css::style_selector::{AtRule, AtRuleKind};
use serial_test::serial;

mod first_value;

#[test]
#[serial]
fn nested_at_rule_pseudo_classes_reference_the_emitted_selector() {
    // Given
    css::atom_hoist::restore_atom_plan(None);
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::reset_file_routes();
    css::file_map::reset_canonical_map();
    css::set_prefix(None);
    let mut sheet = StyleSheet::default();
    let source = r"import {Box} from '@devup-ui/react';
      export const x=<Box _supports={{'(display:grid)':{
        _media:{'(hover:hover)':{_hover:{color:'red'},_focus:{color:'blue'}}}
      }}}/>;";
    // When
    let output = extractor::extract("private.tsx", source, extractor::ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    sheet.update_styles(&output.styles, "private.tsx", false);
    // Then
    let css = sheet.create_css(Some("private.tsx"), false);
    let classes: Vec<&str> = output
        .code
        .split("className=\"")
        .nth(1)
        .and_then(|code| code.split('"').next())
        .unwrap_or_else(|| panic!("{}", output.code))
        .split_whitespace()
        .collect();
    assert_eq!(classes.len(), 2);
    assert_ne!(classes[0], classes[1]);
    assert!(
        css.contains("@supports(display:grid){@media(hover:hover){"),
        "{css}"
    );
    for (selector, color) in [("hover", "red"), ("focus", "blue")] {
        assert!(
            classes
                .iter()
                .any(|name| css.contains(&format!(".{name}:{selector}{{color:{color}}}"))),
            "{css}"
        );
    }
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
}

#[test]
#[serial]
fn content_identities_emit_distinct_rules_for_structural_boundaries() {
    // Given
    css::atom_hoist::restore_atom_plan(None);
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::reset_file_routes();
    css::file_map::reset_canonical_map();
    css::set_prefix(None);
    let at = |kind, outer| {
        Some(StyleSelector::At {
            kind,
            query: "(width:10px)".to_string(),
            selector: None,
            outer,
            file: None,
        })
    };
    let cases = [
        (None, None, 0, None, None),
        (None, None, 0, Some(""), None),
        (None, None, 0, Some("red"), None),
        (None, None, 1, Some("red"), None),
        (None, None, 0, Some("red"), Some(1)),
        (None, Some(""), 0, Some("red"), None),
        (None, Some("cards"), 0, Some("red"), None),
        (
            Some(StyleSelector::Selector("&:hover".into())),
            None,
            0,
            Some("red"),
            None,
        ),
        (at(AtRuleKind::Media, vec![]), None, 0, Some("red"), None),
        (at(AtRuleKind::Supports, vec![]), None, 0, Some("red"), None),
        (
            at(AtRuleKind::Container, vec![]),
            None,
            0,
            Some("red"),
            None,
        ),
        (
            at(
                AtRuleKind::Media,
                vec![AtRule {
                    kind: AtRuleKind::Supports,
                    query: "(display:grid)".into(),
                }],
            ),
            None,
            0,
            Some("red"),
            None,
        ),
        (
            at(
                AtRuleKind::Media,
                vec![AtRule {
                    kind: AtRuleKind::Container,
                    query: "(display:grid)".into(),
                }],
            ),
            None,
            0,
            Some("red"),
            None,
        ),
    ];
    let mut sheet = StyleSheet::default();
    let mut names = BTreeSet::new();
    // When
    for (selector, layer, level, value, order) in cases {
        let key = css::atom_name::selector_key(selector.as_ref(), layer);
        let name = sheet_to_classname(
            "color",
            level,
            value,
            Some(&key),
            order,
            Some("private.tsx"),
        );
        assert!(names.insert(name.clone()), "colliding identity {name}");
        sheet.add_property_with_layer(
            &name,
            "color",
            level,
            value.unwrap_or("var(--x)"),
            selector.as_ref(),
            order,
            Some("private.tsx"),
            layer,
        );
    }
    // Then
    let css = sheet.create_css(Some("private.tsx"), false);
    for name in names {
        assert!(
            css.contains(&format!(".{name}")),
            "missing rule {name}\n{css}"
        );
    }
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
}

#[test]
#[serial]
fn global_selector_owner_is_cleanup_only_not_content_identity() {
    // Given
    css::atom_hoist::restore_atom_plan(None);
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::set_file_routes(std::collections::HashMap::from([
        ("a.tsx".to_string(), std::collections::HashSet::from([0, 1])),
        ("b.tsx".to_string(), std::collections::HashSet::from([0, 1])),
    ]));
    let mut sheet = StyleSheet::default();
    let source = "import {globalCss} from '@devup-ui/react'; globalCss({body:{color:'red'}});";
    let mut identities = BTreeSet::new();
    // When
    for file in ["a.tsx", "b.tsx"] {
        let output = extractor::extract(file, source, extractor::ExtractOption::default())
            .unwrap_or_else(|error| panic!("{error}"));
        sheet.update_styles(&output.styles, file, false);
        for levels in sheet.properties[file].values() {
            for prop in levels.values().flatten() {
                identities.insert(prop.class_name.clone());
            }
        }
    }
    // Then
    assert_eq!(identities.len(), 1);
    let emitted = sheet.create_css(None, false);
    assert!(
        emitted.contains(concat!("body{", "color:red}")),
        "{emitted}"
    );
    sheet.rm_global_css("a.tsx", false);
    assert!(
        sheet
            .create_css(None, false)
            .contains(concat!("body{", "color:red}"))
    );
    let global_key = css::atom_name::selector_key(
        Some(&StyleSelector::Global("body".into(), "b.tsx".into())),
        None,
    );
    let local_key =
        css::atom_name::selector_key(Some(&StyleSelector::Selector("body".into())), None);
    assert_ne!(global_key, local_key);
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::file_routes::reset_file_routes();
}

#[test]
#[serial]
fn shared_imports_and_fonts_signal_changes_only_when_added() {
    // Given
    css::atom_hoist::restore_atom_plan(None);
    css::atom_hoist::set_atom_hoist(Some(2));
    let source = "import {globalCss} from '@devup-ui/react'; globalCss({imports:['base.css'],fontFaces:[{fontFamily:'Fixture',src:'url(font.woff2)'}]});";
    let output = extractor::extract("test.tsx", source, extractor::ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    let mut sheet = StyleSheet::default();
    // When
    let first = sheet.update_styles(&output.styles, "test.tsx", false);
    let repeated = sheet.update_styles(&output.styles, "test.tsx", false);
    // Then
    assert!(first.1);
    assert!(!repeated.1);
    let emitted = sheet.create_css(None, false);
    assert!(emitted.contains("@import \"base.css\";"));
    assert!(emitted.contains("font-family:Fixture"));
    assert!(emitted.contains("src:url(font.woff2)"));
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
}
