use crate::assignment_test_support::{compiled_jsx, evaluate, extracted, lowered};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("export const x=<Box flexDir={['column','row']} display={['none',null,null,'flex']}/>;")]
#[case("const values=['column','row'] as const; export const x=<Box flexDir={values}/>;")]
#[case("export const x=<Box _hover={{flexDir:['column','row']}}/>;")]
#[case("export const x=css({flexDir:['column','row'],display:['none',null,null,'flex']});")]
#[case("export const x=styled('div',{flexDir:['column','row']});")]
#[case(
    "import {jsx} from 'react/jsx-runtime'; export const x=jsx(Box,{flexDir:['column','row']});"
)]
#[serial]
fn static_arrays_emit_only_classes_when_source_is_literal_or_already_folded(#[case] body: &str) {
    // Given: static Footer/Header shapes across the shared extraction surfaces.
    let source = format!("import {{Box,css,styled}} from '@devup-ui/react'; {body}");
    // When: the existing constant processor and extractor compile the source.
    let output = extracted(&source);
    // Then: no runtime tuple or empty inline assignment is introduced for static arrays.
    for marker in [
        "__devupAssignment",
        "__devupLevel",
        "__devupValue",
        "__devupCreation",
        "__devupStyled",
        "...{}",
    ] {
        assert!(!output.code.contains(marker), "{marker}\n{}", output.code);
    }
    let levels = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) if style.property() == "flex-direction" => {
                Some((style.level(), style.value().to_string()))
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        levels,
        std::collections::BTreeSet::from([(0, "column".into()), (1, "row".into())])
    );
}

#[rstest]
#[case("reads.flag ? ['4px','8px'] : ['4px','8px']")]
#[case("[reads.flag ? '4px' : '4px','8px']")]
#[case("[['4px','8px'],['4px','8px']][index()]")]
#[case("[reads.a,'8px']")]
#[case("[`1${index()}`,'8px']")]
#[serial]
fn dynamic_sources_keep_reads_when_emitted_classes_happen_to_be_static(#[case] expression: &str) {
    // Given: runtime getters/controllers whose alternatives can have identical content.
    let setup = "let trace=[]; const reads={get flag(){trace.push('flag');return true},get a(){trace.push('a');return '4px'}};function index(){trace.push('index');return 1}";
    let expected = evaluate(&format!(
        "{setup} const value=({expression});JSON.stringify(trace);"
    ));
    // When: actual lowered assignment code executes.
    let actual = evaluate(&format!(
        "{setup} const node={};JSON.stringify(trace);",
        lowered(expression)
    ));
    // Then: every observable read survives with its exact count and order.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn static_arrays_preserve_adjacent_operands_when_the_element_has_runtime_reads() {
    // Given: static arrays between retained reads and an authored child.
    let source = "import {Box} from '@devup-ui/react';function render(s){return <Box id={s.before} flexDir={['column','row']} title={s.after}>{s.child}</Box>}let trace=[];const s={get before(){trace.push('before');return 1},get after(){trace.push('after');return 2},get child(){trace.push('child');return 3}};render(s);JSON.stringify(trace);";
    // When: compiled JSX runs through the existing native evaluator.
    let actual = evaluate(&compiled_jsx(source));
    // Then: the static fast path cannot reorder adjacent operands.
    assert_eq!(actual, "[\"before\",\"after\",\"child\"]");
}

#[test]
#[serial]
fn coercion_keeps_runtime_reads_when_a_template_contains_a_literal_container() {
    // Given: container-to-string conversion can call the runtime prototype.
    let setup =
        "let trace=[];Array.prototype.toString=function(){trace.push('coercion');return '2'};";
    let expected = evaluate(&format!(
        "{setup} const value=[`1${{[]}}`,'8px'];JSON.stringify(trace);"
    ));
    // When
    let actual = evaluate(&format!(
        "{setup} const node={};JSON.stringify(trace);",
        lowered("[`1${[]}`,'8px']")
    ));
    // Then
    assert_eq!(actual, expected);
}
