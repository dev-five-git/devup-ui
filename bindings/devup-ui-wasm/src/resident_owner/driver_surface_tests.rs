use std::cell::{Cell, RefCell};

use super::driver_native_tests::*;
use crate::resolver::native_tests::{Value, cause};
use serial_test::serial;

#[test]
#[serial]
fn opaque_default_extensions_change_real_render_obligations_with_unresolved_resolver() {
    // Given
    let _fixture = Fixture::new();
    let code = "import Doc from './doc.custom'; export const view=<Doc/>;";
    install(|_, _| Ok(Value::Undefined));
    let control = DRIVER.extract(source("/page.tsx", code), args()).required();
    assert_eq!(control.deferred_defaults(), "[]");
    let mut arguments = args();
    arguments.opaque_defaults = list(vec![Value::text(".custom")]);
    // When
    let output = DRIVER
        .extract(source("/page.tsx", code), arguments)
        .required();
    // Then
    let records: serde_json::Value = serde_json::from_str(&output.deferred_defaults()).required();
    assert_eq!(records[0]["cutRequest"], "./doc.custom");
    assert_eq!(records[0]["cutImporter"], "/page.tsx");
    assert_eq!(output.code(), code);
}

#[test]
#[serial]
fn transitive_resolver_fault_reports_its_importer_and_first_cause() {
    // Given
    let _fixture = Fixture::new();
    install(|_, importer| {
        if importer == &Value::text("/root.tsx") {
            Ok(module("/forward.ts", "export {C} from './missing'"))
        } else {
            assert_eq!(importer, &Value::text("/forward.ts"));
            Err(Value::text("inner failure"))
        }
    });
    // When
    let error = DRIVER.extract(source("/root.tsx", "import {Box} from '@devup-ui/react'; import {C} from './forward'; export const view=<Box color={C}/>;"), args()).err().required();
    // Then
    assert_eq!(
        message(&error),
        "/forward.ts:1:1: module resolver cannot use `./missing` at build time: inner failure"
    );
    assert_eq!(error.get("cause").required(), Value::text("inner failure"));
    assert_eq!(cause(), None);
}

#[test]
#[serial]
fn optional_css_file_selects_only_the_numbered_bucket_and_unknown_means_whole_sheet() {
    // Given
    let _fixture = Fixture::new();
    let mut first = source(
        "first.tsx",
        "import {Box} from '@devup-ui/react'; <Box color='red'/>;",
    );
    first.option.single_css = false;
    DRIVER.extract(first, args()).required();
    let mut second = source(
        "second.tsx",
        "import {Box} from '@devup-ui/react'; <Box color='blue'/>;",
    );
    second.option.single_css = false;
    DRIVER.extract(second, args()).required();
    let number = css::file_map::get_file_num_by_filename("first.tsx");
    // When
    let selected = crate::configuration::css_internal(Some(number), false);
    // Then
    assert!(selected.contains("color:red"));
    assert!(!selected.contains("color:blue"));
    assert_eq!(
        crate::configuration::css_internal(Some(usize::MAX), true),
        crate::configuration::css_internal(None, true)
    );
}

thread_local! {
    static REPORTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CONSOLE_FAILS: Cell<bool> = const { Cell::new(false) };
}
fn console(message: &str) -> Result<(), ()> {
    REPORTS.with_borrow_mut(|messages| messages.push(message.into()));
    if CONSOLE_FAILS.get() { Err(()) } else { Ok(()) }
}

#[test]
fn panic_reporting_formats_once_and_console_failure_does_not_escape() {
    for fails in [false, true] {
        // Given
        REPORTS.with_borrow_mut(Vec::clear);
        CONSOLE_FAILS.set(fails);
        // When
        crate::startup::report("panic payload at source.rs:2:3", console);
        // Then
        assert_eq!(
            REPORTS.with_borrow(Clone::clone),
            ["panic payload at source.rs:2:3"]
        );
    }
}
