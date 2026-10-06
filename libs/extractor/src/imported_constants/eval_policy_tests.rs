use super::exact_tests::{extracted, static_values};
use rstest::rstest;

#[rstest]
#[case("eval('made.p=2');", "made.p")]
#[case("(eval)('made.p=2');", "made.p")]
#[case("eval(input);", "made.p")]
#[case("eval('Math.imul=()=>9');", "Math.imul(1,1)")]
#[case("eval('make=()=>2');", "make()")]
#[case("eval('value=2');", "value")]
#[case("function change(){eval('made.p=2')}change();", "made.p")]
#[case("const change=()=>eval('made.p=2');change();", "made.p")]
#[case("eval('made.p=2');", "read()")]
#[case(r"const alias=made;{const made={ p: 2 };eval('alias.p=2')}", "made.p")]
#[serial_test::serial]
fn lexical_eval_when_values_are_visible_blocks_build_only_reads(
    #[case] hazard: &str,
    #[case] value: &str,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{p:1}};const value=1;let make=()=>1;const read=()=>made.p;{hazard}css({{p:{value}}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{hazard} / {value}: {result:?}");
}

#[rstest]
#[case("(0,eval)('made.p=2');")]
#[case("globalThis.eval('made.p=2');")]
#[case("eval?.('made.p=2');")]
#[case("const invoke=eval;invoke('made.p=2');")]
#[case("eval.call(null,'made.p=2');")]
#[case("eval.apply(null,['made.p=2']);")]
#[case("const invoke=eval.bind(null);invoke('made.p=2');")]
#[case(r"function other(){const made={ p: 2 };eval('made.p=3')}other();")]
#[case(r"{const made={ p: 2 };eval('made.p=3')}")]
#[serial_test::serial]
fn eval_when_indirect_shadowed_or_outside_visibility_keeps_local_object_exact(
    #[case] hazard: &str,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{p:1}};{hazard}css({{p:made.p}});"
    );
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{hazard}: {error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}

#[test]
#[serial_test::serial]
fn lexical_eval_when_after_eager_read_preserves_earlier_snapshot() {
    // Given
    let source = r"import {css} from '@devup-ui/react';const made={ p: 1 };css({ p: made.p });eval('made.p=2');";
    // When
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}

#[test]
#[serial_test::serial]
fn lexical_eval_when_after_deferred_read_keeps_expression_runtime() {
    // Given
    let source = r"import {Box} from '@devup-ui/react';const made={ p: 1 };export const View=()=> <Box p={made.p}/>;eval('made.p=2');";
    // When
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), Vec::<String>::new());
    assert!(
        output.code.contains("made.p") && output.code.contains("--"),
        "{}",
        output.code
    );
}

#[rstest]
#[case(r"const value=Math.imul(1,1);css({ p: value });")]
#[case(r"const child=made.p;css({ p: child });")]
#[case(r"const value=make();css({ p: value });")]
#[serial_test::serial]
fn lexical_eval_when_before_initializers_stops_secondary_folds(#[case] consumer: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{p:1}};let make=()=>1;eval(input);{consumer}"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{consumer}: {result:?}");
}

#[rstest]
#[case("(0,eval)('Math.imul=()=>9');")]
#[case("globalThis.eval('Math.imul=()=>9');")]
#[case("eval?.('Math.imul=()=>9');")]
#[case("const invoke=eval;invoke('Math.imul=()=>9');")]
#[case("eval.call(null,'Math.imul=()=>9');")]
#[case("eval.apply(null,['Math.imul=()=>9']);")]
#[case("const invoke=eval.bind(null);invoke('Math.imul=()=>9');")]
#[case("eval['call'](null,'Math.imul=()=>9');")]
#[case("globalThis['eval']('Math.imul=()=>9');")]
#[case("let invoke=()=>0;invoke=eval;invoke('Math.imul=()=>9');")]
#[serial_test::serial]
fn indirect_eval_when_mutating_globals_blocks_later_math(#[case] hazard: &str) {
    // Given
    let source =
        format!("import {{css}} from '@devup-ui/react';{hazard}css({{p:Math.imul(1,1)}});");
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{hazard}: {result:?}");
}

#[rstest]
#[case("const eval=()=>0;eval('made.p=2');")]
#[case("function other(eval){eval('made.p=2')}other(()=>0);")]
fn shadowed_eval_when_valid_in_script_is_not_a_lexical_barrier(#[case] source: &str) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::cjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When
    let barriers = super::eval_barriers::EvalBarriers::new(&parsed.program);
    // Then
    assert_eq!(
        barriers.affects(None, oxc_span::Span::new(0, u32::MAX)),
        None
    );
}

#[rstest]
#[case("(0,eval)('JSON.stringify=watch');JSON.stringify(made);")]
#[case("globalThis.eval('Object.keys=watch');Object.keys(made);")]
#[case("const invoke=eval;invoke('Object.freeze=watch');Object.freeze(made);watch(made);")]
#[serial_test::serial]
fn indirect_eval_when_builtin_policy_can_change_stops_readonly_exemptions(#[case] hazard: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';const made={{p:1}};{hazard}css({{p:made.p}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{hazard}: {result:?}");
}

#[test]
#[serial_test::serial]
fn lexical_eval_when_rejected_reports_consumer_binding_and_origin() {
    // Given
    let source = r"import {css} from '@devup-ui/react';
const made={ p: 1 };
eval(input);
css({ p: made.p });";
    // When
    let error = extracted(source, "")
        .err()
        .unwrap_or_else(|| panic!("lexical eval read must fail"));
    // Then
    for expected in [
        "/src/App.tsx:4:",
        "/src/App.tsx:3:",
        "`made`",
        "eval(input)",
        "use a direct value",
    ] {
        assert!(error.contains(expected), "{error}");
    }
}
