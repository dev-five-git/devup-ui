use std::{cell::RefCell, rc::Rc};

use super::{driver_native_tests::*, driver_value_tests::CREATED};
use crate::resolver::native_tests::{STATE, Value, cause};
use serial_test::serial;

const VIEW: &str = "import {Box} from '@devup-ui/react'; import {C} from './token'; export const view=<Box color={C}/>;";

#[test]
#[serial]
fn extraction_uses_real_resolver_before_warm_cache_with_both_source_map_modes() {
    for generate in [false, true] {
        // Given
        let _fixture = Fixture::new();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        install(move |request, importer| {
            observed
                .borrow_mut()
                .push((request.clone(), importer.clone()));
            Ok(module("/token.ts", "export const C='red'"))
        });
        let mut input = source("/view.tsx", VIEW);
        input.map_mode = if generate {
            crate::SourceMapMode::Generate
        } else {
            crate::SourceMapMode::Skip
        };
        let output = DRIVER.extract(input, args()).required();
        assert_eq!(output.map().is_some(), generate);
        assert!(output.css().required().contains("color:red"));
        assert_eq!(output.dependencies(), ["/token.ts"]);
        assert!(
            calls
                .borrow()
                .iter()
                .all(|(_, importer)| importer == &Value::text("/view.tsx"))
        );
        let thrown = Value::object(&[("message", Value::text("changed host"))]);
        let original = thrown.clone();
        install(move |_, _| Err(original.clone()));
        // When
        let failure = DRIVER
            .extract(source("/view.tsx", VIEW), args())
            .err()
            .required();
        // Then
        assert_eq!(failure.get("cause").required(), thrown);
        assert!(
            message(&failure).starts_with("/view.tsx:1:1: module resolver cannot use `./token`")
        );
        assert_eq!(cause(), None);
    }
}

#[test]
#[serial]
fn first_fault_shortcircuits_later_callbacks_and_wins_over_both_native_outcomes() {
    for expression in ["<Box color={A} bg={B}/>", "css({color:A,background:B})"] {
        // Given
        let _fixture = Fixture::new();
        let code = format!(
            "import {{Box,css}} from '@devup-ui/react'; import {{A}} from './a'; import {{B}} from './b'; {expression};"
        );
        assert_eq!(
            extractor::extract_with_source_type(
                "/root.tsx",
                &code,
                source("/root.tsx", &code).option,
                false,
                None,
                None
            )
            .is_ok(),
            expression.starts_with('<')
        );
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        install(move |request, _| {
            observed.borrow_mut().push(request.clone());
            Err(Value::text("first"))
        });
        let before = crate::export_sheet_internal().required();
        // When
        let error = DRIVER
            .extract(source("/root.tsx", &code), args())
            .err()
            .required();
        // Then
        assert_eq!(
            message(&error),
            "/root.tsx:1:1: module resolver cannot use `./a` at build time: first"
        );
        assert_eq!(*calls.borrow(), vec![Value::text("./a")]);
        assert_eq!(crate::export_sheet_internal().required(), before);
    }
}

#[test]
#[serial]
fn structural_failure_has_priority_and_retains_exact_first_cause_in_active_export() {
    // Given
    let _fixture = Fixture::new();
    admit();
    install(|_, _| {
        let _fault = reenter();
        Err(Value::Undefined)
    });
    let before = crate::export_sheet_internal().required();
    // When
    let error = DRIVER
        .extract(source("/owner.tsx", VIEW), args())
        .err()
        .required();
    // Then
    assert_eq!(
        error.get("name").required(),
        Value::text("ResidentOwnerError")
    );
    assert_eq!(error.get("cause").required(), Value::Undefined);
    assert_eq!(
        super::driver_value_tests::ERRORS
            .details(&error)
            .get("2")
            .required(),
        Value::text("/owner.tsx")
    );
    assert_eq!(cause(), None);
    assert_eq!(crate::export_sheet_internal().required(), before);
    assert!(crate::resident_owner::check_fault().is_err());
}

#[test]
#[serial]
fn ordered_input_decoding_shortcircuits_and_default_values_skip_host_conversion() {
    for mode in [Value::Undefined, Value::Null] {
        // Given
        let _fixture = Fixture::new();
        let mut arguments = args();
        arguments.opaque_defaults = mode;
        // When
        let output = DRIVER
            .extract(source("plain.ts", "export const x=1;"), arguments)
            .required();
        // Then
        assert!(output.map().is_none());
        assert_eq!(CONVERSIONS.with_borrow(Clone::clone), ["aliases"]);
        assert_eq!(CREATED.get(), 0);
    }
    let _fixture = Fixture::new();
    let mut arguments = args();
    arguments.source_type = Value::text("tsx");
    arguments.aliases = Value::Bool(false);
    let error = DRIVER
        .extract(source("bad.custom", ""), arguments)
        .err()
        .required();
    assert!(message(&error).starts_with("bad.custom:1:1: source type"));
    assert!(CONVERSIONS.with_borrow(Vec::is_empty));
}

#[test]
#[serial]
fn alias_mapping_and_compiled_source_override_reach_real_extraction() {
    // Given
    let _fixture = Fixture::new();
    let mut arguments = args();
    arguments.aliases =
        Value::object(&[("ui", Value::Null), ("styled-lib", Value::text("styled"))]);
    arguments.source_type = Value::text("compiled-mdx");
    arguments.opaque_defaults = list(vec![]);
    // When
    let output = DRIVER.extract(source("page.custom", "import {css} from 'ui'; import styled from 'styled-lib'; const C=styled('div',{color:'blue'}); export const bg=css({background:'red'}); export const view=<C/>;"), arguments).required();
    // Then
    assert!(!output.code().contains("css({"));
    assert!(output.css().required().contains("background:red"));
    assert_eq!(
        CONVERSIONS.with_borrow(Clone::clone),
        ["aliases", "strings"]
    );
}

#[test]
#[serial]
fn conversion_errors_precede_extraction_and_do_not_attach_previous_cause() {
    for (aliases, defaults, expected) in [
        (Value::Bool(false), list(vec![]), vec!["aliases"]),
        (
            Value::object(&[]),
            Value::Bool(false),
            vec!["aliases", "strings"],
        ),
    ] {
        // Given
        let _fixture = Fixture::new();
        STATE.with(|state| state.remember(&Value::text("stale")));
        let mut arguments = args();
        arguments.aliases = aliases;
        arguments.opaque_defaults = defaults;
        // When
        let error = DRIVER
            .extract(source("bad.extension", ""), arguments)
            .err()
            .required();
        // Then
        assert_eq!(CONVERSIONS.with_borrow(Clone::clone), expected);
        assert!(!message(&error).contains("bad.extension"));
        assert_eq!(error.get("cause").required(), Value::Undefined);
        assert_eq!(cause(), None);
    }
}
