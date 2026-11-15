use extractor::ExtractSourceType;
use rstest::rstest;

use super::native_tests::*;

#[rstest]
#[case(Value::Undefined)]
#[case(Value::Null)]
fn absent_source_type_keeps_filename_derived_policy(#[case] source_type: Value) {
    // Given omitted/null source-type input.
    let _fixture = Fixture::new();
    // When the production mapping normalizes that input.
    let result = RESOLVER.source_type(&source_type);
    // Then no explicit override or reflection/diagnostic side effect is introduced.
    assert!(matches!(result, Ok(None)));
    assert!(READS.with_borrow(Vec::is_empty));
    assert_eq!(cause(), None);
}

#[test]
fn valid_source_type_selects_compiled_mdx_not_importer_defaults() {
    // Given the one supported explicit source type.
    let _fixture = Fixture::new();
    // When both root and child callsites use the same production mapping.
    let result = RESOLVER.source_type(&Value::text("compiled-mdx"));
    // Then it selects the typed compiler mode.
    assert!(matches!(result, Ok(Some(ExtractSourceType::CompiledMdx))));
}

#[rstest]
#[case(Value::Bool(false))]
#[case(Value::object(&[]))]
#[case(Value::text(""))]
#[case(Value::text("typescript"))]
fn malformed_source_type_fails_without_turning_validation_into_a_throw(#[case] value: Value) {
    // Given unsupported or non-string source-type input.
    let _fixture = Fixture::new();
    // When the production source-type mapper reads that input.
    let result = RESOLVER.source_type(&value);
    // Then validation rejects it without reflection or a fabricated original exception.
    assert!(result.is_err());
    assert_eq!(cause(), None);
    assert!(READS.with_borrow(Vec::is_empty));
}

#[rstest]
#[case(Value::Bool(false))]
#[case(Value::object(&[]))]
#[case(Value::text(""))]
#[case(Value::text("typescript"))]
fn malformed_child_source_type_fails_after_ordered_required_fields(#[case] value: Value) {
    // Given a valid resolved file with a malformed child source type.
    let _fixture = Fixture::new();
    let module = module();
    module.set("sourceType", value);
    // When the actual decoder reaches the child's parsing mode.
    let result = RESOLVER.call(&callback(Ok(module)), "./tokens", "/src/app.tsx");
    // Then validation rejects it without substituting an importer override or a cause.
    assert!(result.is_err());
    assert_eq!(cause(), None);
    assert_eq!(
        READS.with_borrow(Clone::clone),
        ["ignored", "path", "code", "sourceType"]
    );
}
