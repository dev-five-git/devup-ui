use super::*;

fn raw_evaluated(value: &Expression<'_>, setup: &str) -> (String, (Option<String>, Vec<String>)) {
    let script = format!(
        "const trace=[];{setup};const value=({});JSON.stringify([typeof value,[value,trace]])",
        readable_code(value)
    );
    let mut context = boa_engine::Context::default();
    let result = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    let json = result
        .to_string(&mut context)
        .unwrap_or_else(|error| panic!("JSON string: {error}"))
        .to_std_string_escaped();
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("raw class JSON: {error}"))
}

#[rstest]
#[case("a", "external-a")]
#[case("b", "external-b")]
#[case("missing", "")]
#[serial]
fn literal_class_selection_when_key_is_constant_or_captured_returns_string(
    #[case] key: &str,
    #[case] expected: &str,
    #[values(false, true)] getter: bool,
) {
    // Given: the actual literal class table and a constant key or distinguishable getter.
    let (selection, setup, trace) = if getter {
        (
            "state.key".to_string(),
            format!("const state={{get key(){{trace.push('{key}');return '{key}';}}}}"),
            vec![key.to_string()],
        )
    } else {
        (format!("'{key}'"), String::new(), Vec::new())
    };
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>cx(({{a:'external-a',b:'external-b'}})[{selection}])}}</ClassNames>;"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: the real local compiler supplies the capture and emits the class result.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: even a miss returns a primitive string, with the original span and one key read.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let actual = raw_evaluated(&value, &setup);
    assert_eq!(actual.0, "string");
    assert_eq!(actual.1, (Some(expected.to_string()), trace));
}

#[rstest]
#[case("'missing'", "", Vec::<String>::new())]
#[case(
    "state.key",
    "const state={get key(){trace.push('missing');return 'missing';}}",
    vec!["missing".to_string()]
)]
#[serial]
fn literal_class_selection_when_missing_precedes_tail_omits_absent_token(
    #[case] selection: &str,
    #[case] setup: &str,
    #[case] trace: Vec<String>,
) {
    // Given: the missing selection is composed with an ordinary external tail class.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>cx(({{a:'external-a',b:'external-b'}})[{selection}],'ordinary-tail')}}</ClassNames>;"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: local class composition merges the selected result with the tail.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: only the tail token remains; whitespace layout is not part of this contract.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let (kind, (class, actual_trace)) = raw_evaluated(&value, setup);
    assert_eq!(kind, "string");
    let class = class.unwrap_or_else(|| panic!("primitive class string"));
    assert_eq!(
        class.split_whitespace().collect::<Vec<_>>(),
        vec!["ordinary-tail"]
    );
    assert_eq!(actual_trace, trace);
}

#[rstest]
#[case("??", "a", "external-a")]
#[case("??", "b", "external-b")]
#[case("??", "missing", "fallback")]
#[case("??", "empty", "")]
#[case("||", "a", "external-a")]
#[case("||", "b", "external-b")]
#[case("||", "missing", "fallback")]
#[case("||", "empty", "fallback")]
#[serial]
fn literal_class_selection_when_logical_preserves_original_controller(
    #[case] operator: &str,
    #[case] key: &str,
    #[case] expected: &str,
) {
    // Given: nullish and falsy selections have distinct authored fallback behavior.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>cx(({{a:'external-a',b:'external-b',empty:''}})[state.key]{operator}'fallback')}}</ClassNames>;"
    );
    let setup = format!("const state={{get key(){{trace.push('{key}');return '{key}';}}}}");
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: the existing logical path captures the selector and chooses its class side.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: absence selects fallback, while an empty string remains non-nullish.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let actual = raw_evaluated(&value, &setup);
    assert_eq!(actual.0, "string");
    assert_eq!(
        actual.1,
        (Some(expected.to_string()), vec![key.to_string()])
    );
}

#[test]
#[serial]
fn literal_class_selection_when_missing_tag_collides_with_undefined_key_returns_empty() {
    // Given: an ordinary miss and a distinct literal property named undefined.
    let source = "import {ClassNames} from '@emotion/react';<ClassNames>{({css,cx})=>cx(({a:'external-a',undefined:'collision'})[state.key])}</ClassNames>;";
    let setup = "const state={get key(){trace.push('missing');return 'missing';}}";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    let span = value.span();
    // When: the real local compiler captures the missing selection.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: the absent tag cannot select the undefined property, and the getter runs once.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let actual = raw_evaluated(&value, setup);
    assert_eq!(actual.0, "string");
    assert_eq!(actual.1, (Some(String::new()), vec!["missing".to_string()]));
}

#[test]
#[serial]
fn literal_class_selection_when_saved_tag_is_undefined_string_retains_value() {
    // Given: the real string key undefined selects the distinguishable collision value.
    let source = "import {ClassNames} from '@emotion/react';<ClassNames>{({css,cx})=>cx(({a:'external-a',undefined:'collision'})[state.key])}</ClassNames>;";
    let setup = "const state={get key(){trace.push('undefined');return 'undefined';}}";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    let span = value.span();
    // When: the real local compiler captures the present primitive string tag.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: the selected string survives, without replaying the authored getter.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let actual = raw_evaluated(&value, setup);
    assert_eq!(actual.0, "string");
    assert_eq!(
        actual.1,
        (Some("collision".to_string()), vec!["undefined".to_string()])
    );
}
