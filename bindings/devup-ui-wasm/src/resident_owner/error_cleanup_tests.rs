use css::resident_owner::OwnerFault;
use rstest::rstest;

use super::error_native_tests::{TRANSPORT, Value, setup};

#[rstest]
#[case(Value::Undefined)]
#[case(Value::text("primitive"))]
#[case(Value::new())]
fn cleanup_keeps_unauthenticated_primary_and_cleanup_references(#[case] primary: Value) {
    // Given a caller exception and a distinct cleanup failure.
    setup(None);
    let cleanup = Value::new();
    // When production combines failures.
    let result = TRANSPORT.cleanup(primary.clone(), cleanup.clone());
    // Then neither reference is replaced or granted authority.
    assert_eq!(result.get("cause"), primary);
    assert_eq!(result.get("cleanupCause"), cleanup);
    assert_eq!(TRANSPORT.details(&result), Value::Undefined);
    for key in ["cause", "cleanupCause"] {
        let Some(descriptor) = result.property(key) else {
            panic!("cleanup must define both properties")
        };
        assert!(descriptor.configurable && descriptor.writable && !descriptor.enumerable);
    }
}

#[rstest]
#[case(false)]
#[case(true)]
fn cleanup_propagates_cause_presence_without_inventing_one(#[case] present: bool) {
    // Given authenticated metadata, with absent cause or Some(undefined).
    setup(Some("/src/original-importer.ts"));
    let primary = TRANSPORT.error(OwnerFault::CaptureFailed);
    let primary = if present {
        TRANSPORT.attach_cause(primary, Value::Undefined)
    } else {
        primary
    };
    let metadata = TRANSPORT.details(&primary);
    let cleanup = Value::new();
    // When cleanup wraps the primary.
    let result = TRANSPORT.cleanup(primary.clone(), cleanup.clone());
    // Then all native-selected metadata and the explicit-presence bit survive.
    let copied = TRANSPORT.details(&result);
    for index in [0, 1, 2, 4, 5] {
        assert_eq!(copied.at(index), metadata.at(index));
    }
    assert_eq!(copied.at(3), cleanup);
    assert_eq!(
        copied.at(4),
        if present {
            Value::Bool(true)
        } else {
            Value::Undefined
        }
    );
    assert_eq!(result.get("cause"), primary);
    assert_eq!(result.get("cleanupCause"), cleanup);
    assert_eq!(metadata.at(3), Value::Undefined);
}

#[test]
fn recursive_cleanup_preserves_every_previous_branch_and_original_cause() {
    // Given an authenticated fault, explicit undefined cause and two existing cleanups.
    setup(Some("/src/transitive.ts"));
    let original = TRANSPORT.with_cause(OwnerFault::CaptureFailed, Value::Undefined);
    let first = TRANSPORT.error(OwnerFault::ForeignEngine);
    let second = Value::new();
    let previous = TRANSPORT.cleanup(original.clone(), first.clone());
    let previous = TRANSPORT.cleanup(previous, second.clone());
    let latest = Value::new();
    // When a third cleanup fails while unwinding the existing authenticated tree.
    let result = TRANSPORT.cleanup(previous.clone(), latest.clone());
    // Then recursion adds the new branch without losing any earlier identity.
    assert_eq!(result.get("cause"), previous);
    let outer_cleanup = result.get("cleanupCause");
    let inner_cleanup = outer_cleanup.get("cause");
    let latest_branch = outer_cleanup.get("cleanupCause");
    assert_eq!(latest_branch.get("cause"), second);
    assert_eq!(latest_branch.get("cleanupCause"), latest);
    assert_eq!(inner_cleanup.get("cause"), first);
    assert_eq!(inner_cleanup.get("cleanupCause"), second);
    let metadata = TRANSPORT.details(&result);
    assert_eq!(metadata.at(0), Value::text("CaptureFailed"));
    assert_eq!(metadata.at(2), Value::text("/src/transitive.ts"));
    assert_eq!(metadata.at(3), outer_cleanup);
    assert_eq!(metadata.at(4), Value::Bool(true));
    assert_eq!(metadata.at(5), Value::Undefined);
    assert_eq!(original.get("cause"), Value::Undefined);
    assert!(original.property("cleanupCause").is_none());
}

#[test]
fn completion_combines_failures_using_the_same_cleanup_policy() {
    // Given both an action failure and a different unwind failure.
    setup(None);
    let primary = TRANSPORT.error(OwnerFault::ConfigurationFailed);
    let unwind = Value::new();
    // When production completes the operation.
    let Err(result) = TRANSPORT.complete::<()>(Err(primary.clone()), Err(unwind.clone())) else {
        panic!("two failures must fail completion")
    };
    // Then the primary identity and authenticated classification take precedence.
    assert_eq!(result.get("cause"), primary);
    assert_eq!(result.get("cleanupCause"), unwind);
    assert_eq!(
        TRANSPORT.details(&result).at(0),
        Value::text("ConfigurationFailed")
    );
}
