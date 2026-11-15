use super::driver_native_tests::*;
use crate::{
    driver::ReservationOptions,
    resolver::native_tests::{STATE, Value, cause},
};
use serial_test::serial;

fn options() -> ReservationOptions<'static, Value> {
    ReservationOptions {
        package: "@devup-ui/react",
        single_css: true,
        import_main_css: false,
        aliases: Value::object(&[]),
    }
}

#[test]
#[serial]
fn relevance_uses_alias_fallback_and_resolver_fault_to_request_real_transform() {
    for (code, aliases, expected) in [
        (
            "import {Box} from '@devup-ui/react';",
            Value::Bool(false),
            true,
        ),
        ("export const x=1;", Value::object(&[]), false),
        (
            "import {Box} from 'ui';",
            Value::object(&[("ui", Value::Null)]),
            true,
        ),
        ("import {Box} from './ui';", Value::object(&[]), false),
    ] {
        // Given
        let _fixture = Fixture::new();
        // When
        let actual = DRIVER.relevant("app.tsx", code, "@devup-ui/react", aliases);
        // Then
        assert_eq!(actual, expected);
    }
    let _fixture = Fixture::new();
    install(|_, _| Err(Value::text("host failed")));
    assert!(DRIVER.relevant(
        "app.tsx",
        "import {value} from './unknown';",
        "@devup-ui/react",
        Value::object(&[])
    ));
    assert_eq!(cause(), None);
}

#[test]
#[serial]
fn reservation_rows_get_fresh_causes_and_faulty_rows_rollback_before_suffix() {
    // Given
    let _fixture = Fixture::new();
    let bad = "import {Box} from '@devup-ui/react'; import {C} from './bad'; <Box color={C}/>;";
    let good = "import {Box} from '@devup-ui/react'; import {C} from './good'; <Box color={C}/>;";
    install(|request, importer| {
        if importer == &Value::text("bad.tsx") {
            Err(Value::object(&[("message", Value::text("row fault"))]))
        } else {
            assert_eq!(cause(), None);
            assert_eq!(request, &Value::text("./good"));
            Ok(module("token.ts", "export const C='blue'"))
        }
    });
    let before = crate::export_sheet_internal().required();
    // When
    DRIVER
        .reserve(
            list(vec![row("bad.tsx", bad), row("good.tsx", good)]),
            options(),
        )
        .required();
    // Then
    assert_eq!(crate::get_suppressed_reservation_row_count(), 1);
    assert_eq!(crate::export_sheet_internal().required(), before);
    assert_eq!(cause(), None);
    let output = DRIVER.extract(source("good.tsx", good), args()).required();
    assert!(output.css().required().contains("color:blue"));
    assert_eq!(crate::get_unreserved_class_slot_count(), 0);
}

#[test]
#[serial]
fn structural_reservation_fault_stops_suffix_and_escapes_final_guard() {
    // Given
    let _fixture = Fixture::new();
    admit();
    install(|_, importer| {
        assert_eq!(importer, &Value::text("bad.tsx"));
        reenter();
        Err(Value::text("host cause"))
    });
    let before = crate::export_class_map_internal().required();
    let bad =
        "import {Box} from '@devup-ui/react'; import {C} from './x'; <Box gap={value} color={C}/>;";
    // When
    let error = DRIVER
        .reserve(
            list(vec![row("bad.tsx", bad), row("suffix.tsx", bad)]),
            options(),
        )
        .err()
        .required();
    // Then
    assert_eq!(
        error.get("name").required(),
        Value::text("ResidentOwnerError")
    );
    assert_eq!(error.get("cause").required(), Value::text("host cause"));
    assert_eq!(crate::export_class_map_internal().required(), before);
    assert_eq!(crate::get_suppressed_reservation_row_count(), 0);
}

#[test]
#[serial]
fn reservation_parses_whole_packet_before_options_or_allocation() {
    for rows in [
        Value::Bool(false),
        list(vec![list(vec![Value::text("short")])]),
    ] {
        // Given
        let _fixture = Fixture::new();
        let before = crate::export_file_map_internal().required();
        let mut option = options();
        option.aliases = Value::Bool(false);
        // When
        assert!(DRIVER.reserve(rows, option).is_err());
        // Then
        assert_eq!(CONVERSIONS.with_borrow(Clone::clone), ["rows"]);
        assert_eq!(crate::export_file_map_internal().required(), before);
    }
}

#[test]
#[serial]
fn reservation_without_resolver_reserves_real_rows_and_empty_retry_resets_suppression() {
    // Given
    let _fixture = Fixture::new();
    STATE.with(|state| state.remember(&Value::text("old")));
    let source = "import {Box} from '@devup-ui/react'; export const view=<Box color='red'/>;";
    // When
    DRIVER
        .reserve(list(vec![row("good.tsx", source)]), options())
        .required();
    // Then
    assert_eq!(cause(), None);
    let output = DRIVER
        .extract(
            super::driver_native_tests::source("good.tsx", source),
            args(),
        )
        .required();
    assert!(output.css().required().contains("color:red"));
    assert_eq!(crate::get_unreserved_class_slot_count(), 0);
    DRIVER.reserve(list(vec![]), options()).required();
    assert_eq!(crate::get_suppressed_reservation_row_count(), 0);
}
