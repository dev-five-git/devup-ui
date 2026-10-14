use super::*;
use extraction_rollback::{ExtractionRollback, SheetData};
use std::collections::BTreeSet;

/// Cleans only poison deliberately created by a sheet-lock panic test.
pub(super) struct SheetPoisonCleanup(());

impl SheetPoisonCleanup {
    pub(super) fn new() -> Self {
        assert!(
            !GLOBAL_STYLE_SHEET.is_poisoned(),
            "unexpected prior sheet poison"
        );
        Self(())
    }
}

impl Drop for SheetPoisonCleanup {
    fn drop(&mut self) {
        // The enclosing test scope outlives the intentionally unwound sheet guard.
        GLOBAL_STYLE_SHEET.clear_poison();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Authority {
    classes: HashMap<String, HashMap<String, usize>>,
    files: bimap::BiHashMap<String, usize>,
    originals: BTreeMap<String, u32>,
    plan: Option<BTreeSet<String>>,
    sheet: SheetData,
    theme: String,
}

pub(super) fn authority() -> Authority {
    with_admission(|| Authority {
        classes: css::class_map::get_class_map(),
        files: css::file_map::get_file_map(),
        originals: css::file_map::get_original_ids(),
        plan: css::atom_hoist::atom_plan(),
        sheet: with_style_sheet(SheetData::capture),
        theme: with_style_sheet(|sheet| sheet.theme.to_css()),
    })
}

pub(super) fn compile(file: &str, source: &str) -> Result<Output, String> {
    code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
}

pub(super) fn fixture() {
    reset_build_state_internal();
    set_debug(false);
    let mut theme = sheet::theme::Theme::default();
    let mut colors = sheet::theme::ColorTheme::default();
    colors.add_color("retained", "#123456");
    theme.add_color_theme("default", colors);
    register_theme_internal(theme);
    code_extract_internal(
        "retained.tsx",
        "import {css} from '@devup-ui/react';export const x=css({color:'blue'});",
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("retained fixture extraction failed: {error}"));
    set_class_map(HashMap::from([
        ("empty".into(), HashMap::new()),
        (
            "retained".into(),
            HashMap::from([("non-length".into(), 37)]),
        ),
    ]));
    set_file_map(std::iter::once(("retained.tsx".into(), 29)).collect());
    css::file_map::set_original_ids(BTreeMap::from([("retained.tsx".into(), 42)]));
    css::atom_hoist::restore_atom_plan(None);
    with_style_sheet_mut(|sheet| {
        sheet.atom_plan = Some(BTreeSet::from(["retained.tsx".into()]));
        sheet.source_ids.insert("sheet-only".into(), 91);
        sheet.add_css("raw.tsx", "body{margin:1px}");
        sheet.add_import("raw.tsx", "url('retained.css')");
        sheet.add_font_face(
            "raw.tsx",
            &BTreeMap::from([("font-family".into(), "retained".into())]),
        );
        sheet.add_keyframes(
            "ordered",
            BTreeMap::from([(
                "from".into(),
                vec![
                    ("z-index".into(), "2".into()),
                    ("opacity".into(), "0".into()),
                    ("opacity".into(), "1".into()),
                ],
            )]),
            Some("raw.tsx"),
        );
        let props = sheet
            .properties
            .values_mut()
            .flat_map(BTreeMap::values_mut)
            .flat_map(BTreeMap::values_mut)
            .next()
            .unwrap_or_else(|| panic!("retained fixture properties missing"));
        let mut distinct = props
            .iter()
            .next()
            .unwrap_or_else(|| panic!("retained fixture property record missing"))
            .clone();
        distinct.class_name = "same-ord-distinct-record".into();
        props.insert(distinct);
    });
}

pub(super) fn exact<O>(build: impl FnOnce() -> Result<O, String>) -> Result<O, String> {
    with_admission(|| {
        cache_names::check()?;
        let rollback = ExtractionRollback::capture();
        let result = css::exact_attempt::with_exclusive_attempt(build);
        if result.is_ok() {
            rollback.commit();
        }
        result
    })
}

pub(super) fn mutate_authority() {
    let _file_num = css::file_map::get_file_num_by_filename("tentative.tsx");
    css::file_map::get_or_insert_original_id("tentative.tsx")
        .unwrap_or_else(|error| panic!("tentative original registration failed: {error}"));
    css::atom_hoist::freeze_atom_plan();
    let inner = css::class_map::Attempt::begin();
    css::class_map::with_class_map_mut(|map| {
        map.remove("empty");
        map.insert(
            "retained".into(),
            HashMap::from([("replacement".into(), 99)]),
        );
        map.insert("new-empty".into(), HashMap::new());
    });
    let _name = css::keyframes_to_keyframes_name("inserted", None);
    inner.commit();
    with_style_sheet_mut(|sheet| {
        sheet.names.clear();
        sheet.properties.clear();
        sheet.keyframes.clear();
        sheet.rm_global_css("raw.tsx", false);
        sheet.add_css("tentative.tsx", "body{margin:2px}");
        sheet.atom_plan = None;
        sheet.source_ids.clear();
    });
}

pub(super) fn configure_fresh_plan() {
    set_atom_hoist(Some(1));
    import_file_routes_internal(HashMap::from([(
        "tentative.tsx".into(),
        std::collections::HashSet::from([1]),
    )]));
}
