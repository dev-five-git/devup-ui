use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{KeyframesMap, PropertyMap, StyleSheet, StyleSheetCss, name_registry::NameRegistry};

pub type ClassMap = BTreeMap<String, BTreeMap<String, usize>>;
pub type FileMap = BTreeMap<String, usize>;

#[derive(Debug, Default)]
pub enum CacheRestore {
    #[default]
    Manual,
    Rejected,
    Serialized {
        classes: ClassMap,
        files: FileMap,
    },
}

#[derive(Deserialize)]
struct Snapshot {
    names: NameRegistry,
    #[serde(deserialize_with = "Option::deserialize")]
    atom_plan: Option<BTreeSet<String>>,
    #[serde(deserialize_with = "crate::deserialize_btree_map_u8")]
    properties: BTreeMap<String, PropertyMap>,
    css: BTreeMap<String, BTreeSet<StyleSheetCss>>,
    keyframes: KeyframesMap,
    global_css_files: BTreeSet<String>,
    imports: BTreeMap<String, BTreeSet<String>>,
    font_faces: BTreeMap<String, BTreeSet<BTreeMap<String, String>>>,
    #[serde(rename = "sourceIds")]
    source_ids: BTreeMap<String, u32>,
    #[serde(rename = "classMap")]
    classes: ClassMap,
    #[serde(rename = "fileMap")]
    files: FileMap,
}

fn dense_ids<T: Ord + Copy + TryInto<usize>>(
    values: impl Iterator<Item = T>,
    length: usize,
) -> bool {
    let ids: BTreeSet<_> = values.collect();
    ids.len() == length
        && ids
            .into_iter()
            .enumerate()
            .all(|(expected, id)| id.try_into().is_ok_and(|actual| actual == expected))
}

impl<'de> Deserialize<'de> for StyleSheet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let rejected = || Self {
            cache_restore: CacheRestore::Rejected,
            ..Self::default()
        };
        if value
            .get("atomNamingVersion")
            .and_then(serde_json::Value::as_u64)
            != Some(4)
        {
            return Ok(rejected());
        }
        let snapshot =
            serde_json::from_value::<Snapshot>(value).map_err(serde::de::Error::custom)?;
        if !dense_ids(snapshot.files.values().copied(), snapshot.files.len())
            || !dense_ids(
                snapshot.source_ids.values().copied(),
                snapshot.source_ids.len(),
            )
            || snapshot
                .classes
                .values()
                .any(|classes| !dense_ids(classes.values().copied(), classes.len()))
        {
            return Ok(rejected());
        }
        Ok(Self {
            counter_state: None,
            cache_restore: CacheRestore::Serialized {
                classes: snapshot.classes,
                files: snapshot.files,
            },
            names: snapshot.names,
            atom_plan: snapshot.atom_plan,
            properties: snapshot.properties,
            css: snapshot.css,
            keyframes: snapshot.keyframes,
            global_css_files: snapshot.global_css_files,
            imports: snapshot.imports,
            font_faces: snapshot.font_faces,
            source_ids: snapshot.source_ids,
            theme: crate::theme::Theme::default(),
        })
    }
}

impl std::fmt::Debug for StyleSheet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StyleSheet")
            .field("names", &self.names)
            .field("atom_plan", &self.atom_plan)
            .field("properties", &self.properties)
            .field("css", &self.css)
            .field("keyframes", &self.keyframes)
            .field("global_css_files", &self.global_css_files)
            .field("imports", &self.imports)
            .field("font_faces", &self.font_faces)
            .field("source_ids", &self.source_ids)
            .field("theme", &self.theme)
            .finish()
    }
}

pub(crate) fn export(sheet: &StyleSheet) -> impl Serialize + '_ {
    #[derive(Serialize)]
    struct Export<'a> {
        #[serde(flatten)]
        sheet: &'a StyleSheet,
        #[serde(rename = "atomNamingVersion")]
        version: u8,
        #[serde(rename = "sourceIds")]
        sources: BTreeMap<String, u32>,
        #[serde(rename = "classMap")]
        classes: ClassMap,
        #[serde(rename = "fileMap")]
        files: FileMap,
    }
    css::admission::with_admission(|| Export {
        sheet,
        version: 4,
        sources: css::file_map::get_original_ids(),
        classes: css::class_map::with_class_map(|map| {
            map.iter()
                .map(|(file, classes)| {
                    (
                        file.clone(),
                        classes.iter().map(|(key, id)| (key.clone(), *id)).collect(),
                    )
                })
                .collect()
        }),
        files: css::file_map::with_file_map(|map| {
            map.iter().map(|(file, id)| (file.clone(), *id)).collect()
        }),
    })
}
