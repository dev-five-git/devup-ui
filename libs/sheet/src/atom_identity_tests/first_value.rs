use super::*;

fn token_theme(first: &str) -> Theme {
    serde_json::from_value(serde_json::json!({
        "length": {"default": {"space": [first, "2px"], "zero": ["0px", "4px"], "nullable": [null, "3px", "4px"]}},
        "shadows": {"default": {"card": ["0 1px 2px black", "0 2px 4px black"], "space": ["0 3px 4px black", "0 4px 8px black"]}}
    }))
    .unwrap_or_else(|error| panic!("{error}"))
}

fn token_sheet(atom: bool) -> StyleSheet {
    css::atom_hoist::restore_atom_plan(None);
    css::atom_hoist::set_atom_hoist(atom.then_some(2));
    css::file_routes::reset_file_routes();
    css::file_map::reset_canonical_map();
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    css::set_prefix(None);
    css::debug::set_debug(false);
    let mut sheet = StyleSheet::default();
    sheet.set_theme(token_theme("1px"));
    sheet
}

#[test]
#[serial]
fn first_value_refs_resolve_without_aliasing_responsive_variables() {
    // Given
    let source = r#"import {Box} from '@devup-ui/react'; export const A=<>
      <Box w="$space"/><Box w={["$space"]}/><Box boxShadow={["$card"]}/>
      <Box boxShadow={["$space"]}/><Box w="0px"/><Box w={["$zero"]}/>
      <Box w={["$missing"]}/><Box w={["$nullable"]}/></>;"#;
    for atom in [false, true] {
        let mut sheet = token_sheet(atom);
        // When
        let output = extractor::extract("test.tsx", source, extractor::ExtractOption::default())
            .unwrap_or_else(|error| panic!("{error}"));
        sheet
            .update_styles(&output.styles, "test.tsx", false)
            .unwrap_or_else(|error| panic!("{error}"));
        // Then
        let css = sheet.create_css(Some("test.tsx"), false);
        let classes = crate::sheet_test_code::static_element_classes(
            &output.code,
            &[
                None,
                Some("$space"),
                Some("$card"),
                Some("$space"),
                None,
                Some("$zero"),
                Some("$missing"),
                Some("$nullable"),
            ],
        );
        assert_eq!(classes.len(), 8, "{}", output.code);
        assert_eq!(classes[0].len(), 2);
        for refs in &classes[1..] {
            assert_eq!(refs.len(), 1);
        }
        for (refs, declaration) in classes.iter().zip([
            "width:var(--space)",
            "width:1px",
            "box-shadow:0 1px 2px black",
            "box-shadow:0 3px 4px black",
            "width:0",
            "width:0",
            "width:var(--missing)",
            "width:var(--nullable)",
        ]) {
            for name in refs {
                assert!(
                    css.contains(&format!(".{name}{{{declaration}}}")),
                    "unresolved {name} -> {declaration}\n{}\n{css}",
                    output.code
                );
            }
        }
        assert_ne!(classes[0][0], classes[1][0]);
        assert_eq!(classes[4], classes[5]);
    }
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    StyleSheet::default().set_theme(Theme::default());
}

fn extract_first_value(sheet: &mut StyleSheet) -> String {
    let output = extractor::extract(
        "test.tsx",
        r#"import {Box} from '@devup-ui/react'; export const x=<Box w={["$space"]}/>;"#,
        extractor::ExtractOption::default(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    sheet
        .update_styles(&output.styles, "test.tsx", false)
        .unwrap_or_else(|error| panic!("{error}"));
    let classes = crate::sheet_test_code::static_element_classes(&output.code, &[Some("$space")]);
    assert_eq!(classes[0].len(), 1, "{}", output.code);
    classes[0][0].clone()
}

#[test]
#[serial]
fn first_value_names_track_theme_updates_without_aliasing_stale_literals() {
    // Given
    for atom in [false, true] {
        let mut sheet = token_sheet(atom);
        let old_name = extract_first_value(&mut sheet);
        // When
        sheet.set_theme(token_theme("8px"));
        let updated_name = extract_first_value(&mut sheet);
        // Then
        assert_ne!(old_name, updated_name);
        let css = sheet.create_css(Some("test.tsx"), false);
        assert!(css.contains(&format!(".{old_name}{{width:1px}}")), "{css}");
        assert!(
            css.contains(&format!(".{updated_name}{{width:8px}}")),
            "{css}"
        );
    }
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    StyleSheet::default().set_theme(Theme::default());
}

#[test]
#[serial]
fn first_value_rebuild_references_the_new_default_instead_of_cached_literal() {
    // Given
    for atom in [false, true] {
        let mut old = token_sheet(atom);
        let old_name = extract_first_value(&mut old);
        let mut rebuilt = token_sheet(atom);
        rebuilt.set_theme(token_theme("8px"));
        // When
        let new_name = extract_first_value(&mut rebuilt);
        // Then
        let css = rebuilt.create_css(Some("test.tsx"), false);
        assert!(css.contains(&format!(".{new_name}{{width:8px}}")), "{css}");
        assert!(!css.contains("width:1px"), "{css}");
        if atom {
            assert_ne!(old_name, new_name);
        }
    }
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    StyleSheet::default().set_theme(Theme::default());
}
