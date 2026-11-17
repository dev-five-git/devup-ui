use std::collections::{HashMap, HashSet};

use css::atom_hoist::AtomModeGuard;
use devup_ui_binding_core::{
    ExtractionMode, Output, SourceMapMode, code_extract, import_file_routes_internal,
    reset_state_internal, set_atom_hoist, with_style_sheet,
};
use extractor::ExtractOption;
use serial_test::serial;

fn extract(filename: &str, code: &str, single_css: bool) -> Result<Output, String> {
    code_extract(
        (filename, code),
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "df".to_string(),
            single_css,
            ..ExtractOption::default()
        },
        ExtractionMode {
            source_map: SourceMapMode::Skip,
            resolver: None,
            resolver_check: None,
            import_main_css_in_css: false,
        },
    )
}

fn view(color: &str) -> String {
    format!(
        "import {{ Box }} from '@devup-ui/react'; export const view = <Box styleOrder={{1}} color=\"{color}\" />;"
    )
}

/// The base as a fresh serialization writes it now, under the given atom mode.
fn fresh_base(single_css: bool) -> String {
    let _mode = AtomModeGuard::new(single_css);
    with_style_sheet(|sheet| sheet.create_css(None, false))
}

#[test]
#[serial]
fn css_read_at_once_is_the_sheet_the_transform_left() -> Result<(), String> {
    // Given: a transform that added a style.
    reset_state_internal(|| {});
    let output = extract("red.tsx", &view("red"), true)?;
    let expected = fresh_base(true);

    // When: its CSS is read twice before anything else changes.
    let css = output.css.ok_or("expected CSS")?;

    // Then: both reads are the serialization of that sheet.
    assert_eq!(css.get(), expected);
    assert_eq!(css.get(), expected);
    Ok(())
}

#[test]
#[serial]
fn css_read_after_another_transform_is_the_sheet_its_transform_left() -> Result<(), String> {
    // Given: a transform whose CSS is not read yet.
    reset_state_internal(|| {});
    let red = extract("red.tsx", &view("red"), true)?;
    let expected = fresh_base(true);

    // When: another transform changes the sheet before the read.
    let blue = extract("blue.tsx", &view("blue"), true)?;
    let red = red.css.ok_or("expected CSS")?.get();

    // Then: the late read still holds only what the first transform saw.
    assert_eq!(red, expected);
    assert!(!red.contains("blue"));
    assert!(blue.css.ok_or("expected CSS")?.get().contains("blue"));
    Ok(())
}

#[test]
#[serial]
fn a_dropped_result_is_never_serialized() -> Result<(), String> {
    // Given: a transform whose result is released unread.
    reset_state_internal(|| {});
    drop(extract("red.tsx", &view("red"), true)?);

    // When: another transform changes the sheet.
    let blue = extract("blue.tsx", &view("blue"), true)?;

    // Then: its own CSS holds both styles.
    let css = blue.css.ok_or("expected CSS")?.get();
    assert!(css.contains("red") && css.contains("blue"));
    Ok(())
}

#[test]
#[serial]
fn css_read_after_an_atom_setting_changes_keeps_the_transform_mode() -> Result<(), String> {
    // Given: atom hoisting over two routes, and a per-file transform unread.
    reset_state_internal(|| {});
    import_file_routes_internal(HashMap::from([(
        "shared.tsx".to_string(),
        HashSet::from([0, 1]),
    )]));
    set_atom_hoist(Some(2));
    let shared = extract("shared.tsx", &view("red"), false)?;
    let expected = with_style_sheet(|sheet| sheet.create_css(Some("shared.tsx"), false));

    // When: a single-CSS transform suppresses atom mode, then hoisting stops.
    let single = extract("single.tsx", &view("blue"), true)?;
    let single_expected = fresh_base(true);
    set_atom_hoist(None);

    // Then: each late read is serialized in the mode of its own transform.
    assert_eq!(shared.css.ok_or("expected CSS")?.get(), expected);
    assert_eq!(single.css.ok_or("expected CSS")?.get(), single_expected);
    reset_state_internal(|| {});
    Ok(())
}
