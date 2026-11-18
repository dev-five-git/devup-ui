use super::*;
use crate::{
    ExtractionMode, SourceMapMode, code_extract, get_css, register_theme_internal,
    remove_file_internal, with_style_sheet,
};
use extractor::ExtractOption;
use serial_test::serial;

/// Clean engine state for the test, and only session 0 afterwards.
struct Clean {
    _sheet: SheetGuard,
    _engine: extractor::test_state::TestStateGuard,
}

struct SheetGuard(StyleSheet);

impl Drop for SheetGuard {
    fn drop(&mut self) {
        reset_sessions();
        with_style_sheet_mut(|sheet| std::mem::swap(sheet, &mut self.0));
    }
}

fn clean() -> Clean {
    reset_sessions();
    Clean {
        _sheet: SheetGuard(with_style_sheet_mut(std::mem::take)),
        _engine: extractor::test_state::TestStateGuard::new(),
    }
}

fn extract(filename: &str, color: &str) -> String {
    extract_source(
        filename,
        &format!("import {{Box}} from '@devup-ui/react'; <Box color=\"{color}\" />;"),
    )
}

fn extract_global(filename: &str, color: &str) -> String {
    extract_source(
        filename,
        &format!(
            "import {{globalCss}} from '@devup-ui/react'; globalCss({{ body: {{ color: \"{color}\" }} }});"
        ),
    )
}

fn extract_source(filename: &str, source: &str) -> String {
    code_extract(
        (filename, source),
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        ExtractionMode {
            source_map: SourceMapMode::Skip,
            resolver: None,
            resolver_check: None,
            import_main_css_in_css: false,
        },
    )
    .map_or_else(|error| panic!("{error}"), |output| output.code)
}

#[test]
#[serial]
fn sessions_keep_their_own_numbering_settings_and_sheet() {
    let _clean = clean();
    // Given: session 0 extracted one file under a prefix
    css::set_prefix(Some("zero-".into()));
    extract("zero.tsx", "red");
    let session = create_session();
    // When: another session extracts the same file differently
    let code = with_session(session, || {
        assert_eq!(live_session(), session);
        assert_eq!(css::get_prefix(), None);
        assert_eq!(css::file_map::get_file_map().len(), 0);
        register_theme_internal(
            serde_json::from_str(r##"{"colors":{"default":{"brand":"#fff"}}}"##)
                .unwrap_or_else(|error| panic!("{error}")),
        );
        extract("other.tsx", "blue")
    });
    // Then: each session reads only its own state
    assert_eq!(live_session(), 0);
    assert_eq!(css::get_prefix().as_deref(), Some("zero-"));
    assert!(get_css(None, false).contains("color:red"));
    assert!(!get_css(None, false).contains("color:blue"));
    assert!(with_style_sheet(|sheet| sheet.theme.colors.is_empty()));
    assert!(code.is_ok_and(|code| code.contains("className=\"a\"")));
    let css = with_session(session, || {
        assert!(
            css::file_map::get_file_map()
                .get_by_left("zero.tsx")
                .is_none()
        );
        get_css(None, false)
    });
    assert!(css.is_ok_and(|css| css.contains("color:blue") && css.contains("--brand")));
}

#[test]
#[serial]
fn the_stylex_style_resolution_belongs_to_one_session() {
    use css::style_resolution::{StyleResolution, style_resolution};
    let _clean = clean();
    // Given: session 0 reads stylex.create() under application order
    assert_eq!(
        crate::set_stylex_style_resolution(Some("application-order")),
        Ok(())
    );
    let session = create_session();
    // When: another session runs, Then: it starts from the default policy
    assert_eq!(
        with_session(session, style_resolution),
        Ok(StyleResolution::PropertySpecificity)
    );
    // And: a policy it installs stays in it
    assert_eq!(
        with_session(session, || crate::set_stylex_style_resolution(Some(
            "legacy-expand-shorthands"
        ))),
        Ok(Ok(()))
    );
    assert_eq!(style_resolution(), StyleResolution::ApplicationOrder);
    assert_eq!(select_session(session), Ok(0));
    assert_eq!(style_resolution(), StyleResolution::LegacyExpandShorthands);
    assert_eq!(select_session(0), Ok(session));
    assert_eq!(style_resolution(), StyleResolution::ApplicationOrder);
}

#[test]
#[serial]
fn removing_a_file_forgets_it_only_in_the_live_session() {
    let _clean = clean();
    // Given: two sessions each extracted global CSS from the same filename
    extract_global("shared.tsx", "red");
    let session = create_session();
    assert!(with_session(session, || extract_global("shared.tsx", "blue")).is_ok());
    // When: the live session 0 forgets the file
    assert!(remove_file_internal("shared.tsx"));
    // Then: session 0 lost its styles while the other session kept its own
    assert!(!get_css(None, false).contains("color:red"));
    let css = with_session(session, || get_css(None, false));
    assert!(css.is_ok_and(|css| css.contains("color:blue")));
    assert_eq!(
        with_session(session, || remove_file_internal("shared.tsx")),
        Ok(true)
    );
}

#[test]
#[serial]
fn selecting_a_session_keeps_it_live_until_another_is_selected() {
    let _clean = clean();
    let first = create_session();
    let second = create_session();
    assert_ne!(first, second);
    assert_eq!(select_session(first), Ok(0));
    css::set_prefix(Some("first-".into()));
    assert_eq!(select_session(first), Ok(first));
    assert_eq!(select_session(second), Ok(first));
    assert_eq!(css::get_prefix(), None);
    assert_eq!(select_session(first), Ok(second));
    assert_eq!(css::get_prefix().as_deref(), Some("first-"));
    assert_eq!(select_session(0), Ok(first));
    assert_eq!(css::get_prefix(), None);
}

#[test]
#[serial]
fn unknown_sessions_are_refused() {
    let _clean = clean();
    let message = unknown(42);
    assert_eq!(select_session(42), Err(message.clone()));
    assert_eq!(with_session(42, || ()), Err(message));
    assert_eq!(live_session(), 0);
}

#[test]
#[serial]
fn nested_calls_may_not_switch_sessions() {
    let _clean = clean();
    let session = create_session();
    let nested = with_session(session, || {
        (
            with_session(session, live_session),
            with_session(0, live_session),
            select_session(session),
            select_session(0),
            drop_session(session),
        )
    });
    assert_eq!(
        nested,
        Ok((
            Ok(session),
            Err(busy(session)),
            Ok(session),
            Err(busy(session)),
            Err(busy(session)),
        ))
    );
    assert_eq!(live_session(), 0);
}

#[test]
#[serial]
fn a_panicking_call_still_switches_back() {
    let _clean = clean();
    let session = create_session();
    let result = std::panic::catch_unwind(|| {
        with_session(session, || {
            css::set_prefix(Some("lost-".into()));
            panic!("call failed");
        })
    });
    assert!(result.is_err());
    assert_eq!(live_session(), 0);
    assert_eq!(css::get_prefix(), None);
    assert_eq!(
        with_session(session, css::get_prefix),
        Ok(Some("lost-".into()))
    );
}

#[test]
#[serial]
fn dropping_sessions() {
    let _clean = clean();
    assert_eq!(
        drop_session(0),
        Err("engine session 0 cannot be dropped".to_string())
    );
    let parked = create_session();
    let live = create_session();
    assert_eq!(drop_session(parked), Ok(true));
    assert_eq!(drop_session(parked), Ok(false));
    assert_eq!(select_session(parked), Err(unknown(parked)));
    css::set_prefix(Some("zero-".into()));
    assert_eq!(select_session(live), Ok(0));
    // Dropping the live session makes session 0 live again
    assert_eq!(drop_session(live), Ok(true));
    assert_eq!(live_session(), 0);
    assert_eq!(css::get_prefix().as_deref(), Some("zero-"));
    // Ids are not handed out twice
    assert!(create_session() > live);
}

#[test]
#[serial]
fn caches_follow_the_session_on_each_thread() {
    let _clean = clean();
    let session = create_session();
    assert_eq!(select_session(session), Ok(0));
    // Another thread aligns its own caches when it switches
    let other = std::thread::spawn(move || {
        let selected = (select_session(session), select_session(0));
        CACHES.with_borrow(|caches| (selected, caches.live, caches.parked.len()))
    })
    .join();
    assert!(other.is_ok_and(|other| other == ((Ok(session), Ok(session)), 0, 1)));
    assert_eq!(live_session(), 0);
    CACHES.with_borrow(|caches| assert_eq!((caches.live, caches.parked.len()), (session, 1)));
    assert_eq!(select_session(0), Ok(0));
    CACHES.with_borrow(|caches| assert_eq!((caches.live, caches.parked.len()), (0, 1)));
}

#[test]
#[serial]
fn per_session_values_follow_the_live_session() {
    let _clean = clean();
    let mut values = PerSession::<u8>::default();
    values.set_live(Some(1));
    let session = create_session();
    assert_eq!(
        with_session(session, || {
            let before = values.live();
            values.set_live(Some(2));
            (before, values.live())
        }),
        Ok((None, Some(2)))
    );
    assert_eq!(values.live(), Some(1));
    values.remove(0);
    assert_eq!(values.live(), None);
    values.set_live(Some(3));
    values.set_live(None);
    assert_eq!(values.live(), None);
    assert!(format!("{values:?}").contains('2'));
}

#[test]
#[serial]
fn full_reset_forgets_every_session() {
    let _clean = clean();
    let session = create_session();
    assert_eq!(select_session(session), Ok(0));
    crate::reset_state_internal(|| {});
    assert_eq!(live_session(), 0);
    assert_eq!(select_session(session), Err(unknown(session)));
}
