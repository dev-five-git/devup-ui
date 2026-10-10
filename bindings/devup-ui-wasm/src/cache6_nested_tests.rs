use super::cache6_test_support::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn inner_failure_restores_depth_one_state_when_outer_continues_or_aborts(#[case] commit: bool) {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let before = authority();
    let mut local = None;
    // When
    let result = exact_test_support::exact(|| {
        output("outer", 1);
        let depth_one = authority();
        let inner: Result<(), String> = exact_test_support::exact(|| {
            output("inner", 2);
            Err("depth two failure".into())
        });
        assert_eq!(inner, Err("depth two failure".into()));
        assert_eq!(authority(), depth_one);
        local = Some(depth_one);
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
            local.unwrap_or_else(|| panic!("local"))
        } else {
            before
        }
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn committed_inner_success_is_subordinate_when_outer_errors_or_unwinds(#[case] unwind: bool) {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let before = authority();
    // When
    let result = catch_unwind(AssertUnwindSafe(|| {
        exact_test_support::exact::<()>(|| {
            exact_test_support::exact(|| {
                output("inner", 1);
                Ok(())
            })?;
            assert_ne!(authority(), before);
            assert!(!unwind, "outer unwind");
            Err("outer abort".into())
        })
    }));
    // Then
    match result {
        Ok(value) => assert_eq!(value, Err("outer abort".into())),
        Err(payload) => assert_eq!(payload.downcast_ref::<&str>(), Some(&"outer unwind")),
    }
    assert_eq!(authority(), before);
}
