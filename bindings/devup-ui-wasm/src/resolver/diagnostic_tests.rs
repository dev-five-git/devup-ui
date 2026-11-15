use std::rc::Rc;

use rstest::rstest;

use super::native_tests::*;

#[test]
fn string_exception_diagnostic_never_reflects_or_stringifies() {
    // Given a string thrown by the resolver.
    let _fixture = Fixture::new();
    let original = Value::text("literal cause");
    GETTER.with_borrow_mut(|getter| *getter = Some(Rc::new(|_, _| panic!("unexpected getter"))));
    JSON.with_borrow_mut(|json| *json = Some(Rc::new(|_| panic!("unexpected stringify"))));
    // When production formats that exception.
    let message = RESOLVER.diagnostic(&original);
    // Then the string is the diagnostic but remains independently stored as the cause.
    assert_eq!(message, "literal cause");
    assert_eq!(cause(), Some(original));
    assert!(READS.with_borrow(Vec::is_empty));
    assert_eq!(JSON_CALLS.get(), 0);
}

#[test]
fn string_message_precedes_json_without_replacing_error_identity() {
    // Given an exception exposing a string message.
    let _fixture = Fixture::new();
    let original = Value::object(&[("message", Value::text("message cause"))]);
    JSON.with_borrow_mut(|json| *json = Some(Rc::new(|_| panic!("unexpected stringify"))));
    // When production reads that diagnostic property.
    let message = RESOLVER.diagnostic(&original);
    // Then the diagnostic wins without converting the retained original into text.
    assert_eq!(message, "message cause");
    assert_eq!(cause(), Some(original));
    assert_eq!(READS.with_borrow(Clone::clone), ["message"]);
    assert_eq!(JSON_CALLS.get(), 0);
}

#[rstest]
#[case(Value::object(&[("message", Value::Bool(false))]), "{\"message\":false}")]
#[case(Value::object(&[("tag", Value::Bool(true))]), "{\"tag\":true}")]
#[case(Value::Undefined, "undefined")]
#[case(Value::Null, "null")]
#[case(Value::Bool(true), "true")]
fn nonstring_message_uses_json_fallback_with_original_still_present(
    #[case] original: Value,
    #[case] expected: &str,
) {
    // Given a non-string exception/message including explicit undefined.
    let _fixture = Fixture::new();
    // When production reaches raw JSON.stringify and string conversion.
    let message = RESOLVER.diagnostic(&original);
    // Then fallback text never erases original identity or presence.
    assert_eq!(message, expected);
    assert_eq!(cause(), Some(original));
    assert_eq!(JSON_CALLS.get(), 1);
}

#[test]
fn throwing_message_getter_does_not_replace_original_with_secondary_throw() {
    // Given an original exception and a hostile diagnostic getter.
    let _fixture = Fixture::new();
    let original = Value::object(&[("tag", Value::Bool(true))]);
    let secondary = Value::object(&[]);
    GETTER.with_borrow_mut(|getter| *getter = Some(Rc::new(move |_, _| Err(secondary.clone()))));
    // When the diagnostic ignores that getter failure and stringifies the original.
    let message = RESOLVER.diagnostic(&original);
    // Then it retains the original, not the getter's new thrown reference.
    assert_eq!(message, "{\"tag\":true}");
    assert_eq!(cause(), Some(original));
}

#[rstest]
#[case(Ok(Value::Undefined), "undefined")]
#[case(Err(Value::object(&[])), "<unprintable JavaScript exception>")]
fn stringify_failure_or_undefined_result_preserves_first_original(
    #[case] result: Result<Value, Value>,
    #[case] expected: &str,
) {
    // Given a host stringify operation returning undefined or throwing.
    let _fixture = Fixture::new();
    let original = Value::object(&[]);
    JSON.with_borrow_mut(|json| *json = Some(Rc::new(move |_| result.clone())));
    // When production selects its existing diagnostic fallback.
    let message = RESOLVER.diagnostic(&original);
    // Then the fallback string remains separate from the pending original cause.
    assert_eq!(message, expected);
    assert_eq!(cause(), Some(original));
    assert_eq!(JSON_CALLS.get(), 1);
}

#[test]
fn hostile_message_reentry_preserves_first_cause_and_origin_without_borrows() {
    // Given a transitive resolver exception whose message getter reenters resolution.
    let _fixture = Fixture::new();
    let original = Value::object(&[]);
    let target = original.clone();
    GETTER.with_borrow_mut(|getter| {
        *getter = Some(Rc::new(move |value, key| {
            if *value == target && key == "message" {
                assert_eq!(cause(), Some(target.clone()));
                assert_eq!(importer(), Some("/src/outer.ts".into()));
                let nested = callback(Err(Value::object(&[(
                    "message",
                    Value::text("nested failure"),
                )])));
                assert!(RESOLVER.call(&nested, "./inner", "/src/inner.ts").is_err());
                STATE.with(|state| {
                    assert_eq!(*state.current.borrow(), Some("/src/outer.ts".into()));
                });
                return Err(Value::object(&[]));
            }
            value.get(key)
        }));
    });
    // When the actual callback/decoder/diagnostic methods perform the reentrant path.
    let result = RESOLVER.call(&callback(Err(original.clone())), "./outer", "/src/outer.ts");
    // Then borrowed state never masks the original and the first origin stays authoritative.
    assert!(matches!(result, Err(message) if message == "{}"));
    assert_eq!(cause(), Some(original));
    assert_eq!(importer(), Some("/src/outer.ts".into()));
    STATE.with(|state| assert_eq!(*state.current.borrow(), None));
}

#[test]
fn stringify_reentry_can_read_and_restore_current_importer_without_changing_first_cause() {
    // Given a stringify operation which makes a nested resolver call.
    let _fixture = Fixture::new();
    let original = Value::object(&[]);
    JSON.with_borrow_mut(|json| {
        *json = Some(Rc::new(|_| {
            let nested = callback(Err(Value::text("nested string cause")));
            assert!(RESOLVER.call(&nested, "./inner", "/src/inner.ts").is_err());
            STATE.with(|state| assert_eq!(*state.current.borrow(), Some("/src/outer.ts".into())));
            Ok(Value::Undefined)
        }));
    });
    // When the production diagnostic reaches the host stringify operation.
    let result = RESOLVER.call(&callback(Err(original.clone())), "./outer", "/src/outer.ts");
    // Then origin/cause remain the first outer failure, not the nested diagnostic string.
    assert!(matches!(result, Err(message) if message == "undefined"));
    assert_eq!(cause(), Some(original));
    assert_eq!(importer(), Some("/src/outer.ts".into()));
}
