pub(super) use super::*;
pub(super) use extractor::{
    counter_test_support::{FixtureRequest, with_original},
    extract_style::{
        ExtractDynamicStyle, ExtractKeyframes, extract_static_style::ExtractStaticStyle,
    },
};
pub(super) use prospective_output::{OutputMetadata, OutputRoute, ProspectiveSheet};
pub(super) use rstest::rstest;
pub(super) use serial_test::serial;
pub(super) use sheet::{
    cache_snapshot::{ClassMap, FileMap},
    counter_kernel::{CounterSheet, UpdateError, UpdateRequest},
    live_checkpoint::LiveCheckpoint,
    snapshot6::{self, EvidenceError},
};
pub(super) use std::collections::BTreeSet;

pub(super) struct Guard;
impl Guard {
    pub(super) fn new() -> Self {
        fresh();
        Self
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        fresh();
        GLOBAL_STYLE_SHEET.clear_poison();
    }
}
pub(super) fn fresh() {
    reset_build_state_internal();
    set_debug(false);
    register_theme_internal(sheet::theme::Theme::default());
}
pub(super) fn ir(file: &str, family: u8) -> FxHashSet<ExtractStyleValue> {
    with_original(
        FixtureRequest {
            filename: file,
            source: "a\nbcd",
        },
        || {
            let item = match family {
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
                    "heading",
                    0,
                    None,
                )),
                4 | 5 => {
                    let mut frames = ExtractKeyframes::default();
                    let member = ExtractStaticStyle::new("opacity", "0", 0, None);
                    let mut members = vec![member.clone(), member];
                    if family == 5 {
                        members.push(ExtractStaticStyle::new("typography", "heading", 0, None));
                    }
                    frames.keyframes.insert("from".into(), members);
                    frames.keyframes.insert("to".into(), vec![]);
                    ExtractStyleValue::Keyframes(frames)
                }
                _ => panic!("family"),
            };
            FxHashSet::from_iter([item])
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"))
}
pub(super) fn metadata() -> OutputMetadata {
    OutputMetadata {
        code: "fixture metadata, not Counter JS".into(),
        map: Some("fixture-map".into()),
        css_file: Some("fixture.css".into()),
        dependencies: vec!["tokens.ts".into()],
    }
}
pub(super) fn update(
    items: &FxHashSet<ExtractStyleValue>,
    route: OutputRoute<'_>,
) -> Result<Output, UpdateError<String>> {
    with_admission(|| {
        let rollback = extraction_rollback::ExtractionRollback::capture();
        let result = css::exact_attempt::with_exclusive_attempt(|| {
            with_style_sheet_mut(|sheet| {
                CounterSheet::new(sheet).with_attempt(|attempt| {
                    attempt
                        .prepare(
                            items,
                            UpdateRequest {
                                raw_source: route.raw_source,
                                single_css: route.single_css,
                            },
                        )?
                        .finish(|sheet, effects| {
                            Ok(Output::from_prospective(
                                metadata(),
                                ProspectiveSheet { sheet, effects },
                                route,
                            ))
                        })
                })
            })
        });
        if result.is_ok() {
            rollback.commit();
        }
        result
    })
}
pub(super) fn output(file: &str, family: u8) -> ObservedOutput {
    let items = ir(file, family);
    let output = update(
        &items,
        OutputRoute {
            raw_source: file,
            single_css: false,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    ObservedOutput::capture(&output, file)
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ObservedOutput {
    code: String,
    map: Option<String>,
    css_file: Option<String>,
    dependencies: Vec<String>,
    css: Option<String>,
    base: bool,
    global_css: String,
    local_css: String,
}
impl ObservedOutput {
    pub(super) fn capture(output: &Output, file: &str) -> Self {
        Self {
            code: output.code(),
            map: output.map(),
            css_file: output.css_file(),
            dependencies: output.dependencies(),
            css: output.css(),
            base: output.updated_base_style(),
            global_css: with_style_sheet(|sheet| sheet.create_css(None, false)),
            local_css: with_style_sheet(|sheet| {
                sheet.create_css(Some(&css::file_map::canonical(file)), false)
            }),
        }
    }
}
pub(super) fn encoded() -> Vec<u8> {
    with_style_sheet_mut(|sheet| {
        serde_json::to_vec(
            &CounterSheet::new(sheet)
                .export_snapshot6()
                .unwrap_or_else(|error| panic!("{error:?}")),
        )
        .unwrap_or_else(|error| panic!("{error:?}"))
    })
}
pub(super) fn maps() -> (ClassMap, FileMap) {
    let maps = cache6_session::Maps::capture();
    (maps.classes, maps.files)
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Authority {
    sheet: LiveCheckpoint,
    maps: cache6_session::Maps,
    theme: String,
}
pub(super) fn authority() -> Authority {
    Authority {
        sheet: with_style_sheet(LiveCheckpoint::capture),
        maps: cache6_session::Maps::capture(),
        theme: with_style_sheet(|sheet| sheet.theme.to_css()),
    }
}
pub(super) fn companions(maps: &(ClassMap, FileMap), reversed: bool) {
    if reversed {
        cache6_restore::files(Some(maps.1.clone())).unwrap_or_else(|error| panic!("{error:?}"));
        cache6_restore::classes(Some(maps.0.clone())).unwrap_or_else(|error| panic!("{error:?}"));
    } else {
        cache6_restore::classes(Some(maps.0.clone())).unwrap_or_else(|error| panic!("{error:?}"));
        cache6_restore::files(Some(maps.1.clone())).unwrap_or_else(|error| panic!("{error:?}"));
    }
}
pub(super) fn configure_typography() {
    let mut theme = sheet::theme::Theme::default();
    let frame = sheet::theme::Typography {
        font_size: Some("14px".into()),
        font_family: None,
        font_style: None,
        font_weight: None,
        line_height: None,
        letter_spacing: None,
        text_transform: None,
    };
    theme.typography.insert(
        "heading".into(),
        sheet::theme::Typographies(vec![Some(frame), None]),
    );
    register_theme_internal(theme);
}
