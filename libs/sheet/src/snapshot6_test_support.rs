pub(super) use super::super::authentic_support::*;
pub(super) use super::{EvidenceError, parse, validate_snapshot};
pub(super) use crate::StyleSheet;
pub(super) use serde_json::{Value, json};

pub(super) fn family_sheet(kind: u8) -> StyleSheet {
    let mut sheet = StyleSheet::default();
    sheet.set_theme(crate::theme::Theme {
        typography: BTreeMap::from([(
            "heading".into(),
            crate::theme::Typographies(vec![
                Some(frame("14px", Some("400"))),
                None,
                Some(frame("24px", None)),
            ]),
        )]),
        ..Default::default()
    });
    let items = fixture("a", || {
        let member = ExtractStaticStyle::new("opacity", "0", 0, None);
        let mut frames = ExtractKeyframes::default();
        frames
            .keyframes
            .insert("from".into(), vec![member.clone(), member]);
        frames.keyframes.insert("to".into(), vec![]);
        styles([match kind {
            0 => ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None)),
            1 => ExtractStyleValue::Dynamic(
                ExtractDynamicStyle::new("color", 0, "tone", None).at_role(2, 1),
            ),
            2 => ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                "color",
                0,
                "tone !important",
                None,
            )),
            3 => ExtractStyleValue::Static(ExtractStaticStyle::new(
                "typography",
                "heading|font-weight:2",
                0,
                None,
            )),
            4 => ExtractStyleValue::Keyframes(frames),
            5 => {
                frames
                    .keyframes
                    .get_mut("from")
                    .required("frame")
                    .push(ExtractStaticStyle::new("typography", "heading", 0, None));
                ExtractStyleValue::Keyframes(frames)
            }
            _ => panic!("family"),
        }])
    });
    update(&mut sheet, &items).required("genuine update");
    sheet
}

pub(super) fn encoded(sheet: &mut StyleSheet) -> Vec<u8> {
    serde_json::to_vec(
        &CounterSheet::new(sheet)
            .export_snapshot6()
            .required("export6"),
    )
    .required("serialize6")
}
pub(super) fn packet(sheet: &mut StyleSheet) -> Value {
    serde_json::from_slice(&encoded(sheet)).required("test-only inspection")
}
pub(super) fn admit(value: Value) -> Result<super::ValidatedSnapshot, EvidenceError> {
    validate_snapshot(parse(&serde_json::to_vec(&value).required("test packet"))?)
}
pub(super) fn fresh() {
    css::class_map::reset_class_map();
    css::file_map::set_file_map(Default::default());
    css::file_map::set_original_ids(BTreeMap::new());
    css::atom_hoist::restore_atom_plan(None);
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Observed {
    live: State,
    source_ids: BTreeMap<String, u32>,
    canonical: HashMap<String, String>,
}
pub(super) fn observe(sheet: &StyleSheet) -> Observed {
    Observed {
        live: capture(sheet),
        source_ids: sheet.source_ids.clone(),
        canonical: css::file_map::get_canonical_map(),
    }
}

pub(super) fn witness(value: &mut Value) -> &mut Value {
    value["evidence"]["counters"]["D9-0"]["0"]
        .as_array_mut()
        .required("witnesses")
        .first_mut()
        .required("witness")
}
