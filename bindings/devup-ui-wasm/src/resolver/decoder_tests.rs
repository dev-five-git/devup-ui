use std::rc::Rc;

use extractor::{ExtractSourceType, ModuleResolution};
use rstest::rstest;

use super::native_tests::*;

#[rstest]
#[case(Value::Null)]
#[case(Value::Undefined)]
fn unresolved_callback_values_do_not_read_result_properties(#[case] value: Value) {
    // Given a callback returning an ordinary unresolved value.
    let _fixture = Fixture::new();
    // When the production resolver decoder runs.
    let result = RESOLVER.call(&callback(Ok(value)), "./tokens", "/src/app.tsx");
    // Then unresolved is not a malformed object and no getter is touched.
    assert!(matches!(result, Ok(None)));
    assert!(READS.with_borrow(Vec::is_empty));
    assert_eq!(cause(), None);
    assert_eq!(importer(), None);
}

#[rstest]
#[case(Value::Bool(false))]
#[case(Value::Bool(true))]
#[case(Value::text("primitive"))]
fn primitive_callback_result_is_rejected_without_inventing_an_original_cause(#[case] value: Value) {
    // Given a resolved result that is not an object.
    let _fixture = Fixture::new();
    // When production checks the host boundary shape.
    let result = RESOLVER.call(&callback(Ok(value)), "./tokens", "/src/app.tsx");
    // Then validation fails before reflection and retains no unrelated thrown value.
    assert!(matches!(result, Err(message) if message == "resolver result must be an object"));
    assert!(READS.with_borrow(Vec::is_empty));
    assert_eq!(cause(), None);
}

#[rstest]
#[case(Value::Undefined)]
#[case(Value::object(&[("message", Value::text("original failure"))]))]
fn callback_exception_preserves_original_reference_and_transitive_origin(#[case] original: Value) {
    // Given a callback throwing an exact value from a transitive importer.
    let _fixture = Fixture::new();
    // When call_module_resolver's production replacement formats the failure.
    let result = RESOLVER.call(
        &callback(Err(original.clone())),
        "./broken",
        "/src/transitive.ts",
    );
    // Then diagnostics do not substitute their string for the original cause.
    assert!(result.is_err());
    assert_eq!(cause(), Some(original));
    assert_eq!(importer(), Some("/src/transitive.ts".into()));
    STATE.with(|state| assert_eq!(*state.current.borrow(), None));
}

#[test]
fn ignored_result_short_circuits_every_file_getter() {
    // Given ignored=true and hostile path/code/sourceType getters.
    let _fixture = Fixture::new();
    let value = Value::object(&[("ignored", Value::Bool(true))]);
    GETTER.with_borrow_mut(|getter| {
        *getter = Some(Rc::new(|value, key| {
            assert_eq!(key, "ignored", "ignored modules must not read file fields");
            value.get(key)
        }));
    });
    // When the actual decoder reads the discriminator.
    let result = RESOLVER.call(&callback(Ok(value)), "native-alias", "/src/app.tsx");
    // Then it returns the ignored module without touching any other getter.
    assert!(matches!(result, Ok(Some(ModuleResolution::Ignored))));
    assert_eq!(READS.with_borrow(Clone::clone), ["ignored"]);
}

#[rstest]
#[case(Value::Bool(false))]
#[case(Value::Null)]
#[case(Value::text("true"))]
#[case(Value::object(&[]))]
fn malformed_ignored_discriminator_is_not_treated_as_a_resolved_file(#[case] ignored: Value) {
    // Given an explicit discriminator other than true/undefined.
    let _fixture = Fixture::new();
    let value = module();
    value.set("ignored", ignored);
    // When the shared decoder applies the ignored contract.
    let result = RESOLVER.call(&callback(Ok(value)), "./tokens", "/src/app.tsx");
    // Then the malformed discriminator fails before required fields.
    assert!(
        matches!(result, Err(message) if message == "resolver field `ignored` must be true or undefined")
    );
    assert_eq!(READS.with_borrow(Clone::clone), ["ignored"]);
    assert_eq!(cause(), None);
}

#[rstest]
#[case("path", Value::Undefined, vec!["ignored", "path"])]
#[case("path", Value::Bool(false), vec!["ignored", "path"])]
#[case("code", Value::Undefined, vec!["ignored", "path", "code"])]
#[case("code", Value::Null, vec!["ignored", "path", "code"])]
fn required_string_field_failure_stops_ordered_decoding(
    #[case] field: &str,
    #[case] value: Value,
    #[case] reads: Vec<&str>,
) {
    // Given an absent or non-string required file field.
    let _fixture = Fixture::new();
    let module = module();
    module.set(field, value);
    // When the actual decoder reaches that field.
    let result = RESOLVER.call(&callback(Ok(module)), "./tokens", "/src/app.tsx");
    // Then later getters are untouched and no original exception is fabricated.
    assert!(
        matches!(result, Err(message) if message == format!("resolver field `{field}` must be a string"))
    );
    assert_eq!(READS.with_borrow(Clone::clone), reads);
    assert_eq!(cause(), None);
}

#[rstest]
#[case("ignored")]
#[case("path")]
#[case("code")]
#[case("sourceType")]
fn property_failure_keeps_exact_original_value_and_field_context(#[case] field: &'static str) {
    // Given a getter throwing an identity-bearing original exception.
    let _fixture = Fixture::new();
    let module = module();
    let target = module.clone();
    let original = Value::object(&[("message", Value::text("getter failure"))]);
    let thrown = original.clone();
    GETTER.with_borrow_mut(|getter| {
        *getter = Some(Rc::new(move |value, key| {
            if *value == target && key == field {
                Err(thrown.clone())
            } else {
                value.get(key)
            }
        }));
    });
    // When production reflects that selected field.
    let result = RESOLVER.call(&callback(Ok(module)), "./tokens", "/src/transitive.ts");
    // Then the diagnostic location and stored original are independent.
    assert!(
        matches!(result, Err(message) if message == format!("reading resolver field `{field}`: getter failure"))
    );
    assert_eq!(cause(), Some(original));
    assert_eq!(importer(), Some("/src/transitive.ts".into()));
    assert_eq!(
        READS.with_borrow(|reads| reads.last().cloned()),
        Some("message".into())
    );
}

#[rstest]
#[case(Value::Undefined, false)]
#[case(Value::Null, false)]
#[case(Value::text("compiled-mdx"), true)]
fn resolved_file_preserves_callback_inputs_fields_and_own_source_type(
    #[case] source_type: Value,
    #[case] compiled: bool,
) {
    // Given a complete module with its own source-type override.
    let _fixture = Fixture::new();
    let value = module();
    value.set("sourceType", source_type);
    let callback: Callback = Rc::new(move |specifier, importer| {
        assert_eq!(*specifier, Value::text("./tokens"));
        assert_eq!(*importer, Value::text("/src/app.tsx"));
        Ok(value.clone())
    });
    // When the shared production method calls and decodes the module.
    let result = RESOLVER.call(&callback, "./tokens", "/src/app.tsx");
    // Then path/code and the child's own parsing mode are retained in order.
    let Ok(Some(ModuleResolution::Resolved(module))) = result else {
        panic!("valid module rejected")
    };
    assert_eq!(module.path, "/src/token.ts");
    assert_eq!(module.code, "export const C = 'red';");
    assert_eq!(
        matches!(module.source_type, Some(ExtractSourceType::CompiledMdx)),
        compiled
    );
    assert_eq!(
        READS.with_borrow(Clone::clone),
        ["ignored", "path", "code", "sourceType"]
    );
}

#[test]
fn property_throwing_undefined_is_not_an_absent_field_or_absent_cause() {
    // Given an ignored discriminator getter throwing undefined, not returning it.
    let _fixture = Fixture::new();
    GETTER.with_borrow_mut(|getter| {
        *getter = Some(Rc::new(|value, key| {
            if key == "ignored" {
                Err(Value::Undefined)
            } else {
                value.get(key)
            }
        }));
    });
    // When the production field decoder observes the host failure.
    let result = RESOLVER.call(&callback(Ok(module())), "./tokens", "/src/app.tsx");
    // Then the error context and explicit original cause presence both survive.
    assert!(
        matches!(result, Err(message) if message == "reading resolver field `ignored`: undefined")
    );
    assert_eq!(cause(), Some(Value::Undefined));
    assert_eq!(importer(), Some("/src/app.tsx".into()));
}
