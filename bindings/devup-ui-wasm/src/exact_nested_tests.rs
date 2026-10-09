use super::*;
use exact_test_support::{authority, compile, exact, fixture, mutate_authority};
use rstest::rstest;
use serial_test::serial;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn real_inner_parser_error_restores_local_authority_when_outer_finishes(#[case] commit: bool) {
    // Given
    fixture();
    let source = "import {Box} from '@devup-ui/react';const x=<Box";
    let expected = compile("broken.tsx", source)
        .err()
        .unwrap_or_else(|| panic!("broken parser fixture unexpectedly compiled"));
    assert!(expected.contains("Parser panicked"));
    let before = authority();
    let mut committed = None;
    // When: the helper is only the outer protocol; the inner is the real binding.
    let result = exact(|| {
        mutate_authority();
        let local = authority();
        let inner = match catch_unwind(AssertUnwindSafe(|| compile("broken.tsx", source))) {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        };
        assert_eq!(inner.err(), Some(expected));
        assert_eq!(authority(), local);
        committed = Some(local);
        if commit {
            Ok(())
        } else {
            Err("outer abort".into())
        }
    });
    // Then
    assert_eq!(
        result,
        if commit {
            Ok(())
        } else {
            Err("outer abort".into())
        }
    );
    assert_eq!(
        authority(),
        if commit {
            committed.unwrap_or_else(|| panic!("committed authority capture missing"))
        } else {
            before
        }
    );
    assert_eq!(cache_names::check(), Ok(()));
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn real_inner_resolver_unwind_preserves_original_panic_when_outer_finishes(#[case] commit: bool) {
    // Given
    fixture();
    let before = authority();
    let mut committed = None;
    let resolver =
        |_: &str, _: &str| -> Option<ResolvedModule> { panic!("nested resolver unwind") };
    // When
    let result = exact(|| {
        mutate_authority();
        let local = authority();
        let panic = match catch_unwind(AssertUnwindSafe(|| {
            code_extract_with_modules_internal(
                "inner.tsx",
                "import {Box} from '@devup-ui/react';import {COLOR} from './tokens';export const x=<Box color={COLOR}/>;",
                "@devup-ui/react",
                "df".into(),
                false,
                false,
                false,
                HashMap::new(),
                &resolver,
            )
        })) {
            Err(payload) => payload,
            Ok(_) => panic!("nested resolver did not panic"),
        };
        assert_eq!(
            panic.downcast_ref::<&str>(),
            Some(&"nested resolver unwind")
        );
        assert_eq!(authority(), local);
        committed = Some(local);
        if commit {
            Ok(())
        } else {
            Err("outer abort".into())
        }
    });
    // Then
    assert_eq!(
        result,
        if commit {
            Ok(())
        } else {
            Err("outer abort".into())
        }
    );
    assert_eq!(
        authority(),
        if commit {
            committed.unwrap_or_else(|| panic!("committed authority capture missing"))
        } else {
            before
        }
    );
    assert_eq!(cache_names::check(), Ok(()));
    reset_build_state_internal();
}

#[test]
#[serial]
fn real_inner_compile_success_is_reversed_when_outer_aborts() {
    // Given
    fixture();
    seed_file_map(vec!["inner.tsx".into()]);
    let before = authority();
    // When
    let result: Result<(), String> = exact(|| {
        let output = compile(
            "inner.tsx",
            "import {Box} from '@devup-ui/react';export const x=<Box w='12px'/>;",
        )
        .unwrap_or_else(|error| panic!("inner compile failed: {error}"));
        assert!(
            output
                .css()
                .unwrap_or_else(|| panic!("inner CSS missing"))
                .contains("width:12px")
        );
        assert_ne!(authority(), before);
        Err("outer abort".into())
    });
    // Then
    assert_eq!(result, Err("outer abort".into()));
    assert_eq!(authority(), before);
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn native_outer_resolver_can_call_real_nested_compiler(#[case] commit: bool) {
    // Given
    fixture();
    let before = authority();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let resolver_calls = std::rc::Rc::clone(&calls);
    let resolver = move |_: &str, _: &str| {
        resolver_calls.set(
            resolver_calls
                .get()
                .checked_add(1)
                .unwrap_or_else(|| panic!("resolver call counter overflow")),
        );
        let local = authority();
        let error = compile(
            "nested-broken.tsx",
            "import {Box} from '@devup-ui/react';const x=<Box",
        )
        .err()
        .unwrap_or_else(|| panic!("nested parser fixture unexpectedly compiled"));
        assert!(error.contains("Parser panicked"));
        assert_eq!(authority(), local);
        Some(ResolvedModule {
            path: "tokens.ts".into(),
            code: "export const COLOR='red';".into(),
        })
    };
    let source = if commit {
        "import {Box} from '@devup-ui/react';import {COLOR} from './tokens';export const x=<Box color={COLOR}/>;"
    } else {
        "import {Box,css} from '@devup-ui/react';import {COLOR} from './tokens';export const x=<Box color={COLOR}/>;export const y=css(runtime());"
    };
    // When: both outer and inner now enter the production common boundary.
    let result = code_extract_with_modules_internal(
        "outer.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
        &resolver,
    );
    // Then
    assert!(calls.get() > 0);
    assert_eq!(result.is_ok(), commit);
    if commit {
        assert!(
            result
                .unwrap_or_else(|error| panic!("outer compile failed: {error}"))
                .css()
                .unwrap_or_else(|| panic!("outer CSS missing"))
                .contains("color:red")
        );
        assert_ne!(authority(), before);
    } else {
        assert!(
            result
                .err()
                .unwrap_or_else(|| panic!("outer invalid fixture unexpectedly compiled"))
                .contains("build time")
        );
        assert_eq!(authority(), before);
    }
    reset_build_state_internal();
}
