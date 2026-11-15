use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};
use sheet::StyleSheet;

#[derive(Default)]
enum CurrentAtomPlan {
    #[default]
    Missing,
    Captured(Option<BTreeSet<String>>),
}

impl<'de> Deserialize<'de> for CurrentAtomPlan {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::deserialize(deserializer).map(Self::Captured)
    }
}

/// Flat public sheet transport with independent extraction and emission authority.
#[derive(Deserialize)]
pub(crate) struct SheetCheckpoint {
    #[serde(flatten)]
    sheet: StyleSheet,
    #[serde(default, rename = "currentAtomPlan")]
    current_atom_plan: CurrentAtomPlan,
}

impl From<StyleSheet> for SheetCheckpoint {
    fn from(sheet: StyleSheet) -> Self {
        Self {
            sheet,
            current_atom_plan: CurrentAtomPlan::Missing,
        }
    }
}

impl SheetCheckpoint {
    pub(crate) fn restore(&self) {
        import_sheet_checkpoint_internal(Self {
            sheet: StyleSheet {
                atom_plan: self.sheet.atom_plan.clone(),
                properties: self.sheet.properties.clone(),
                css: self
                    .sheet
                    .css
                    .iter()
                    .map(|(file, records)| {
                        (
                            file.clone(),
                            records
                                .iter()
                                .map(|record| sheet::StyleSheetCss {
                                    css: record.css.clone(),
                                })
                                .collect(),
                        )
                    })
                    .collect(),
                keyframes: self.sheet.keyframes.clone(),
                global_css_files: self.sheet.global_css_files.clone(),
                imports: self.sheet.imports.clone(),
                font_faces: self.sheet.font_faces.clone(),
                layer_orders: self.sheet.layer_orders.clone(),
                theme: sheet::theme::Theme::default(),
            },
            current_atom_plan: match &self.current_atom_plan {
                CurrentAtomPlan::Missing => CurrentAtomPlan::Missing,
                CurrentAtomPlan::Captured(plan) => CurrentAtomPlan::Captured(plan.clone()),
            },
        });
    }
}

pub(crate) fn import_sheet_checkpoint_internal(checkpoint: SheetCheckpoint) {
    crate::configuration_state::replaced();
    let plan = match checkpoint.current_atom_plan {
        // Persisted exports predating this metadata stored both roles in atom_plan.
        CurrentAtomPlan::Missing => checkpoint.sheet.atom_plan.clone(),
        CurrentAtomPlan::Captured(plan) => plan,
    };
    css::atom_hoist::restore_atom_plan(plan);
    crate::with_style_sheet_mut(|sheet| *sheet = checkpoint.sheet);
}

pub(crate) fn export(sheet: &StyleSheet) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct Export<S> {
        #[serde(flatten)]
        sheet: S,
        #[serde(rename = "currentAtomPlan")]
        current_atom_plan: Option<BTreeSet<String>>,
    }
    serde_json::to_string(&Export {
        sheet: sheet.export_snapshot(),
        current_atom_plan: css::atom_hoist::atom_plan(),
    })
}
