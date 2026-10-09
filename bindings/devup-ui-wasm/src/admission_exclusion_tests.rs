use super::*;
use exact_test_support::{exact, fixture, mutate_authority};
use rstest::rstest;
use serial_test::serial;
use std::sync::mpsc::{TryRecvError, channel};

#[derive(Clone, Copy, Debug)]
enum Gateway {
    ExportSheet,
    GetCss,
    Theme,
    ClassCompanion,
    FileCompanion,
    SheetImport,
    Reset,
    Seed,
    Prefix,
    Debug,
    NamingRoot,
    Canonical,
    Routes,
    Hoist,
    Shorthands,
    Resolver,
    NativeTheme,
    NativeExport,
}

impl Gateway {
    fn call(self) {
        match self {
            Self::ExportSheet => {
                export_sheet_internal()
                    .unwrap_or_else(|error| panic!("gateway sheet export failed: {error}"));
            }
            Self::GetCss => {
                get_css_internal(None, false)
                    .unwrap_or_else(|error| panic!("gateway CSS read failed: {error}"));
            }
            Self::Theme => register_theme_internal(sheet::theme::Theme::default()),
            Self::ClassCompanion => cache_restore::classes(None),
            Self::FileCompanion => cache_restore::files(None),
            Self::SheetImport => {
                import_sheet_internal(StyleSheet::default())
                    .unwrap_or_else(|error| panic!("gateway sheet import failed: {error}"));
            }
            Self::Reset => reset_build_state(),
            Self::Seed => seed_file_map(vec!["worker.tsx".into()]),
            Self::Prefix => set_prefix(Some("worker".into())),
            Self::Debug => set_debug(false),
            Self::NamingRoot => set_naming_root(None, None),
            Self::Canonical => import_canonical_map_internal(HashMap::new()),
            Self::Routes => import_file_routes_internal(HashMap::new()),
            Self::Hoist => set_atom_hoist(None),
            Self::Shorthands => register_shorthands_internal(BTreeMap::new()),
            Self::Resolver => set_module_resolver_internal(None),
            Self::NativeTheme => StyleSheet::default().set_theme(sheet::theme::Theme::default()),
            Self::NativeExport => {
                serde_json::to_string(&StyleSheet::default().export_snapshot())
                    .unwrap_or_else(|error| panic!("native export encoding failed: {error}"));
            }
        }
    }
}

#[rstest]
#[case(Gateway::ExportSheet)]
#[case(Gateway::GetCss)]
#[case(Gateway::Theme)]
#[case(Gateway::ClassCompanion)]
#[case(Gateway::FileCompanion)]
#[case(Gateway::SheetImport)]
#[case(Gateway::Reset)]
#[case(Gateway::Seed)]
#[case(Gateway::Prefix)]
#[case(Gateway::Debug)]
#[case(Gateway::NamingRoot)]
#[case(Gateway::Canonical)]
#[case(Gateway::Routes)]
#[case(Gateway::Hoist)]
#[case(Gateway::Shorthands)]
#[case(Gateway::Resolver)]
#[case(Gateway::NativeTheme)]
#[case(Gateway::NativeExport)]
#[serial]
fn public_gateway_waits_for_exact_release(
    #[case] gateway: Gateway,
    #[values(false, true)] commit: bool,
) {
    // Given
    fixture();
    let (done, received) = channel();
    // When
    std::thread::scope(|scope| {
        let result = exact(|| {
            mutate_authority();
            let contender = scope.spawn(move || {
                gateway.call();
                done.send(())
                    .unwrap_or_else(|error| panic!("gateway completion send failed: {error}"));
            });
            css::admission::observer::wait_for_thread(contender.thread().id());
            assert_eq!(received.try_recv(), Err(TryRecvError::Empty));
            if commit { Ok(()) } else { Err("abort".into()) }
        });
        // Then
        assert_eq!(result, if commit { Ok(()) } else { Err("abort".into()) });
        received
            .recv()
            .unwrap_or_else(|error| panic!("gateway completion receive failed: {error}"));
    });
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn export_observes_only_aborted_or_committed_sheet(#[case] commit: bool) {
    // Given
    fixture();
    let before = with_style_sheet(extraction_rollback::SheetData::capture);
    let (done, received) = channel();
    // When
    std::thread::scope(|scope| {
        let mut committed = None;
        let result = exact(|| {
            mutate_authority();
            committed = Some(with_style_sheet(extraction_rollback::SheetData::capture));
            let contender = scope.spawn(move || {
                let exported = export_sheet_internal()
                    .unwrap_or_else(|error| panic!("contender sheet export failed: {error}"));
                let sheet = with_style_sheet(extraction_rollback::SheetData::capture);
                done.send((exported, sheet))
                    .unwrap_or_else(|error| panic!("export observation send failed: {error}"));
            });
            css::admission::observer::wait_for_thread(contender.thread().id());
            assert_eq!(received.try_recv(), Err(TryRecvError::Empty));
            if commit { Ok(()) } else { Err("abort".into()) }
        });
        // Then
        assert_eq!(result.is_ok(), commit);
        let (exported, observed) = received
            .recv()
            .unwrap_or_else(|error| panic!("export observation receive failed: {error}"));
        assert_eq!(
            observed,
            if commit {
                committed.unwrap_or_else(|| panic!("committed sheet capture missing"))
            } else {
                before
            }
        );
        assert_eq!(
            exported,
            export_sheet_internal()
                .unwrap_or_else(|error| panic!("released sheet export failed: {error}"))
        );
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn admission_precedes_sheet_lock_for_sheet_to_css_callbacks() {
    // Given
    fixture();
    let (done, received) = channel();
    // When
    std::thread::scope(|scope| {
        with_admission(|| {
            let contender = scope.spawn(move || {
                with_style_sheet_mut(|sheet| {
                    let _name = css::keyframes_to_keyframes_name("worker", None);
                    sheet.add_css("worker.tsx", "body{margin:3px}");
                });
                done.send(()).unwrap_or_else(|error| {
                    panic!("sheet callback completion send failed: {error}")
                });
            });
            css::admission::observer::wait_for_thread(contender.thread().id());
            with_style_sheet(|sheet| assert!(sheet.css.contains_key("raw.tsx")));
        });
        // Then
        received
            .recv()
            .unwrap_or_else(|error| panic!("sheet callback completion receive failed: {error}"));
    });
    assert!(with_style_sheet(|sheet| sheet
        .css
        .contains_key("worker.tsx")));
    reset_build_state_internal();
}
