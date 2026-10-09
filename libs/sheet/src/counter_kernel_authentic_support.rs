pub(super) use super::{CounterSheet, UpdateEffects, UpdateError, UpdateRequest};
use crate::StyleSheet;
pub(super) use crate::counter_fixture_support::{fixture, state};
pub(super) use extractor::extract_style::{
    ExtractDynamicStyle, ExtractKeyframes, extract_static_style::ExtractStaticStyle,
    extract_style_value::ExtractStyleValue,
};
use rustc_hash::FxHashSet;
pub(super) use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) fn styles(
    items: impl IntoIterator<Item = ExtractStyleValue>,
) -> FxHashSet<ExtractStyleValue> {
    items.into_iter().collect()
}

pub(super) fn update(
    sheet: &mut StyleSheet,
    items: &FxHashSet<ExtractStyleValue>,
) -> Result<UpdateEffects, UpdateError<()>> {
    CounterSheet::new(sheet).with_attempt(|attempt| {
        attempt
            .prepare(
                items,
                UpdateRequest {
                    raw_source: "a",
                    single_css: false,
                },
            )?
            .finish(|_, effects| Ok(effects))
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct State {
    classes: HashMap<String, HashMap<String, usize>>,
    files: BTreeMap<String, usize>,
    originals: BTreeMap<String, u32>,
    plan: Option<BTreeSet<String>>,
    sheet: serde_json::Value,
    theme: String,
    cache_restore: String,
    owned: Option<super::state::CounterState>,
}
pub(super) fn capture(sheet: &StyleSheet) -> State {
    State {
        classes: css::class_map::get_class_map(),
        files: css::file_map::get_file_map().into_iter().collect(),
        originals: css::file_map::get_original_ids(),
        plan: css::atom_hoist::atom_plan(),
        sheet: serde_json::to_value(sheet).required("sheet serialization"),
        theme: format!("{:?}", sheet.theme),
        cache_restore: format!("{:?}", sheet.cache_restore),
        owned: sheet.counter_state.clone(),
    }
}

pub(super) struct Presets(BTreeMap<String, Vec<(u8, String, String)>>);
pub(super) trait Required {
    type Value;
    fn required(self, context: &str) -> Self::Value;
}
impl<T> Required for Option<T> {
    type Value = T;
    fn required(self, context: &str) -> T {
        match self {
            Some(value) => value,
            None => panic!("missing test fixture: {context}"),
        }
    }
}
impl<T, E: std::fmt::Debug> Required for Result<T, E> {
    type Value = T;
    fn required(self, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("{context}: {error:?}"),
        }
    }
}
pub(super) fn frame(size: &str, weight: Option<&str>) -> crate::theme::Typography {
    crate::theme::Typography {
        font_family: None,
        font_size: Some(size.into()),
        font_style: None,
        font_weight: weight.map(str::to_string),
        line_height: None,
        letter_spacing: None,
        text_transform: None,
    }
}
impl Presets {
    pub(super) fn save() -> Self {
        Self(
            css::theme_tokens::get_typography_keys()
                .into_iter()
                .map(|key| {
                    let declarations = css::content_typography::declarations(&key, 0);
                    (key, declarations)
                })
                .collect(),
        )
    }
}
impl Drop for Presets {
    fn drop(&mut self) {
        css::content_typography::set(std::mem::take(&mut self.0));
    }
}
