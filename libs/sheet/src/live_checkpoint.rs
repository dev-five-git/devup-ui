//! Owned live rollback data, never cache admission authority.
use crate::{
    KeyframesMap, PropertyMap, StyleSheet, StyleSheetCss,
    cache_snapshot::{CacheRestore, ClassMap, FileMap},
    counter_kernel::state::CounterState,
    name_registry::NameRegistry,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "live_checkpoint_tests.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq)]
enum Bookkeeping {
    Manual,
    Rejected,
    Serialized { classes: ClassMap, files: FileMap },
}

/// Infallible full-Eq capture of existing data, including invalid or rejected BASE.
/// No constructor from parts, serialization, clone or allocator authority exists.
/// ```compile_fail,E0277
/// use sheet::live_checkpoint::LiveCheckpoint;
/// fn cloned<T: Clone>() {}
/// cloned::<LiveCheckpoint>();
/// ```
/// ```compile_fail,E0277
/// use sheet::live_checkpoint::LiveCheckpoint;
/// fn defaulted<T: Default>() {}
/// defaulted::<LiveCheckpoint>();
/// ```
/// ```compile_fail,E0277
/// use sheet::live_checkpoint::LiveCheckpoint;
/// fn encoded<T: serde::Serialize>() {}
/// encoded::<LiveCheckpoint>();
/// ```
/// ```compile_fail,E0277
/// use sheet::live_checkpoint::LiveCheckpoint;
/// fn decoded<T: serde::de::DeserializeOwned>() {}
/// decoded::<LiveCheckpoint>();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct LiveCheckpoint {
    names: NameRegistry,
    atom_plan: Option<BTreeSet<String>>,
    properties: BTreeMap<String, PropertyMap>,
    css: BTreeMap<String, BTreeSet<StyleSheetCss>>,
    keyframes: KeyframesMap,
    global_css_files: BTreeSet<String>,
    imports: BTreeMap<String, BTreeSet<String>>,
    font_faces: BTreeMap<String, BTreeSet<BTreeMap<String, String>>>,
    source_ids: BTreeMap<String, u32>,
    state: Option<CounterState>,
    bookkeeping: Bookkeeping,
}

impl LiveCheckpoint {
    /// Copy existing live data without checking or granting provenance.
    #[must_use]
    pub fn capture(sheet: &StyleSheet) -> Self {
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
            state: sheet.counter_state.clone(),
            bookkeeping: match &sheet.cache_restore {
                CacheRestore::Manual => Bookkeeping::Manual,
                CacheRestore::Rejected => Bookkeeping::Rejected,
                CacheRestore::Serialized { classes, files } => Bookkeeping::Serialized {
                    classes: classes.clone(),
                    files: files.clone(),
                },
            },
        }
    }

    /// Consume captured data into the target, leaving its current Theme untouched.
    /// CSS maps/configuration remain the enclosing transaction's responsibility.
    pub fn restore(self, sheet: &mut StyleSheet) {
        sheet.names = self.names;
        sheet.atom_plan = self.atom_plan;
        sheet.properties = self.properties;
        sheet.css = self.css;
        sheet.keyframes = self.keyframes;
        sheet.global_css_files = self.global_css_files;
        sheet.imports = self.imports;
        sheet.font_faces = self.font_faces;
        sheet.source_ids = self.source_ids;
        sheet.counter_state = self.state;
        sheet.cache_restore = match self.bookkeeping {
            Bookkeeping::Manual => CacheRestore::Manual,
            Bookkeeping::Rejected => CacheRestore::Rejected,
            Bookkeeping::Serialized { classes, files } => {
                CacheRestore::Serialized { classes, files }
            }
        };
    }
}
