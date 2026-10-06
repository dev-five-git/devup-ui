use crate::{ExtractOption, ExtractStyleValue};
use rstest::rstest;

#[rstest]
#[case("log")]
#[case("info")]
#[case("warn")]
#[case("error")]
#[case("debug")]
#[case("trace")]
#[case("dir")]
#[case("table")]
#[case("assert")]
#[case("count")]
#[case("countReset")]
#[case("time")]
#[case("timeLog")]
#[case("timeEnd")]
#[case("group")]
#[case("groupCollapsed")]
#[case("groupEnd")]
#[serial_test::serial]
fn console_builtin_when_given_plain_data_preserves_exact_reads(#[case] method: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{p:1}};console.{method}(made);css({{p:made.p}});"
    );
    // When
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let output = crate::extract("/src/App.tsx", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("plain console arguments are read-only: {error}"));
    // Then
    let values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec!["4px".to_string()]);
}

#[rstest]
#[case("const made={p:1,get value(){this.p=2;return 0}};console.log(made);")]
#[case("const made={p:1,toString(){this.p=2;return ''}};console.info(made);")]
#[case("const made={ p: 1 };const console={log(value){value.p=2}};console.log(made);")]
#[serial_test::serial]
fn console_call_when_user_code_may_run_does_not_prove_exactness(#[case] declarations: &str) {
    // Given
    let source = format!("import {{css}} from '@devup-ui/react';{declarations}css({{p:made.p}});");
    // When
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let output = crate::extract("/src/App.tsx", &source, ExtractOption::default());
    // Then
    assert!(output.is_err(), "{output:?}");
}

#[test]
fn local_call_when_wrapped_in_a_type_assertion_has_no_mutable_receiver() {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let source = "const make=()=>({ p:1 });(make as (()=>object))();";
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    // When
    let found = crate::mutations::uses(&parsed.program, &|_| false, None);
    // Then
    assert_eq!(found.len(), 0);
}
