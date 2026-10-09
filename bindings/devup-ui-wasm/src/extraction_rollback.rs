use std::collections::{BTreeMap, BTreeSet};

use rustc_hash::FxHashSet;
use sheet::{StyleSheet, StyleSheetCss, StyleSheetProperty, name_registry::NameRegistry};

use crate::{with_style_sheet, with_style_sheet_mut};

type Properties = BTreeMap<String, BTreeMap<u8, BTreeMap<u8, FxHashSet<StyleSheetProperty>>>>;
type Keyframes = BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<(String, String)>>>>;

/// Typed live records; no cache admission, serialization or lossy Ord conversion.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SheetData {
    names: NameRegistry,
    atom_plan: Option<BTreeSet<String>>,
    properties: Properties,
    css: BTreeMap<String, BTreeSet<StyleSheetCss>>,
    keyframes: Keyframes,
    global_css_files: BTreeSet<String>,
    imports: BTreeMap<String, BTreeSet<String>>,
    font_faces: BTreeMap<String, BTreeSet<BTreeMap<String, String>>>,
    source_ids: BTreeMap<String, u32>,
}

impl SheetData {
    pub(crate) fn capture(sheet: &StyleSheet) -> Self {
        Self {
            names: sheet.names.clone(),
            atom_plan: sheet.atom_plan.clone(),
            properties: sheet.properties.clone(),
            css: sheet
                .css
                .iter()
                .map(|(file, rules)| {
                    (
                        file.clone(),
                        rules
                            .iter()
                            .map(|rule| StyleSheetCss {
                                css: rule.css.clone(),
                            })
                            .collect(),
                    )
                })
                .collect(),
            keyframes: sheet.keyframes.clone(),
            global_css_files: sheet.global_css_files.clone(),
            imports: sheet.imports.clone(),
            font_faces: sheet.font_faces.clone(),
            source_ids: sheet.source_ids.clone(),
        }
    }

    fn restore(self, sheet: &mut StyleSheet) {
        sheet.names = self.names;
        sheet.atom_plan = self.atom_plan;
        sheet.properties = self.properties;
        sheet.css = self.css;
        sheet.keyframes = self.keyframes;
        sheet.global_css_files = self.global_css_files;
        sheet.imports = self.imports;
        sheet.font_faces = self.font_faces;
        sheet.source_ids = self.source_ids;
    }
}

/// Lives outside CSS exact ownership, inside admission, so CSS abort runs first.
/// CSS owns its maps even when an enclosing exact scope remains active.
pub(crate) struct ExtractionRollback(Option<SheetData>);

impl ExtractionRollback {
    pub(crate) fn capture() -> Self {
        Self(Some(with_style_sheet(SheetData::capture)))
    }

    pub(crate) fn commit(mut self) {
        self.0 = None;
    }
}

impl Drop for ExtractionRollback {
    fn drop(&mut self) {
        if let Some(before) = self.0.take() {
            with_style_sheet_mut(|sheet| before.restore(sheet));
        }
    }
}
