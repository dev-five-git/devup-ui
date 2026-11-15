use std::rc::Rc;

use css::resident_owner::OwnerFault;
use rstest::rstest;

use super::{
    decision::Faults,
    error_native_tests::{CREATED, TRANSPORT, Value, setup},
};

#[rstest]
#[case(None)]
#[case(Some("/src/transitive.ts"))]
fn error_authenticates_category_message_and_importer(#[case] importer: Option<&str>) {
    // Given a fault and the current resolver context.
    setup(importer);
    let fault = OwnerFault::ForeignEngine;
    // When production creates the error.
    let error = TRANSPORT.error(fault);
    // Then metadata and public diagnostics agree, without a cause-presence marker.
    let details = TRANSPORT.details(&error);
    assert_eq!(details.at(0), Value::text("ForeignEngine"));
    assert_eq!(details.at(1), Value::text(&fault.to_string()));
    assert_eq!(
        details.at(2),
        importer.map_or(Value::Undefined, Value::text)
    );
    assert_eq!(details.at(4), Value::Undefined);
    assert_eq!(error.get("message"), details.at(1));
    assert_eq!(error.get("name"), Value::text("ResidentOwnerError"));
}

#[rstest]
#[case(Value::Undefined)]
#[case(Value::Bool(true))]
#[case(Value::text("ResidentOwnerError"))]
#[case(Value::new())]
fn details_rejects_unauthenticated_values(#[case] value: Value) {
    // Given an unregistered value, including object/name lookalikes.
    setup(None);
    // When its authority is queried.
    let details = TRANSPORT.details(&value);
    // Then only the registry, not its representation, confers authority.
    assert_eq!(details, Value::Undefined);
}

#[rstest]
#[case(false)]
#[case(true)]
fn attach_cause_preserves_identity_and_explicit_presence(#[case] undefined: bool) {
    // Given an authenticated fault and an original thrown reference or undefined.
    setup(None);
    let error = TRANSPORT.error(OwnerFault::CaptureFailed);
    let original = error.clone();
    let before = TRANSPORT.details(&error);
    let cause = if undefined {
        Value::Undefined
    } else {
        Value::new()
    };
    // When production attaches the cause.
    let result = TRANSPORT.attach_cause(error, cause.clone());
    // Then it retains the thrown identity and copies metadata, even for undefined.
    assert_eq!(result, original);
    let Some(property) = result.property("cause") else {
        panic!("cause must be an own property")
    };
    assert_eq!(property.value, cause);
    assert!(property.configurable && property.writable && !property.enumerable);
    let after = TRANSPORT.details(&result);
    assert_ne!(before, after);
    assert_eq!(before.at(4), Value::Undefined);
    assert_eq!(after.at(4), Value::Bool(true));
    assert_eq!(after.at(5), cause);
}

#[test]
fn attaching_to_an_unregistered_error_does_not_authenticate_it() {
    // Given a caller-owned error-shaped object.
    setup(None);
    let error = Value::new();
    let cause = Value::new();
    // When production attaches a resolver cause to it.
    let result = TRANSPORT.attach_cause(error.clone(), cause.clone());
    // Then the reference and descriptor survive but authority is not invented.
    assert_eq!(result, error);
    assert_eq!(result.get("cause"), cause);
    assert_eq!(TRANSPORT.details(&result), Value::Undefined);
}

#[test]
fn with_cause_authenticates_the_wrapper_not_the_original() {
    // Given an original host exception.
    setup(None);
    let original = Value::new();
    // When production maps the phase fault.
    let result = TRANSPORT.with_cause(OwnerFault::CaptureFailed, original.clone());
    // Then the wrapper is authoritative and the original is untouched.
    assert_eq!(
        TRANSPORT.details(&result).at(0),
        Value::text("CaptureFailed")
    );
    assert_eq!(result.get("cause"), original);
    assert!(original.property("cause").is_none());
    assert_eq!(TRANSPORT.details(&original), Value::Undefined);
}

#[test]
fn registry_does_not_keep_authenticated_errors_alive() {
    // Given an authenticated fault with no external references after this scope.
    setup(None);
    let weak = {
        let error = TRANSPORT.error(OwnerFault::ClosedOwner);
        Rc::downgrade(error.object())
    };
    // When weak reachability is observed.
    let retained = weak.upgrade();
    // Then authentication alone does not retain the error identity.
    assert!(retained.is_none());
}

#[test]
fn completion_success_does_not_construct_an_error() {
    // Given successful work and unwind.
    setup(None);
    // When production completes the operation.
    let result = TRANSPORT.complete(Ok(17), Ok(()));
    // Then success is unchanged without Error/stack allocation.
    assert_eq!(result, Ok(17));
    assert_eq!(CREATED.get(), 0);
}

#[rstest]
#[case(true)]
#[case(false)]
fn completion_rethrows_exact_single_failure(#[case] action_failed: bool) {
    // Given one failure, possibly already authenticated.
    setup(None);
    let primary = TRANSPORT.error(OwnerFault::ConfigurationFailed);
    let (action, unwind) = if action_failed {
        (Err(primary.clone()), Ok(()))
    } else {
        (Ok(3), Err(primary.clone()))
    };
    let before = CREATED.get();
    // When production chooses primary versus unwind.
    let result = TRANSPORT.complete(action, unwind);
    // Then the exact reference is rethrown without a replacement error.
    assert_eq!(result, Err(primary));
    assert_eq!(CREATED.get(), before);
}

#[rstest]
#[case(None)]
#[case(Some(OwnerFault::CleanupFailed))]
fn fault_mapping_preserves_primary_category(#[case] cleanup: Option<OwnerFault>) {
    // Given classified primary and optional cleanup faults.
    setup(None);
    // When production transports the classification.
    let result = TRANSPORT.faults(Faults {
        primary: OwnerFault::ClosedOwner,
        cleanup,
    });
    // Then native-selected primary metadata survives either path.
    assert_eq!(TRANSPORT.details(&result).at(0), Value::text("ClosedOwner"));
    assert_eq!(result.property("cleanupCause").is_some(), cleanup.is_some());
}
