use crate::assignment_test_support::{compiled_jsx, evaluate, extracted, lowered};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("flag ? selected : absent", "const flag=true, selected='4px';")]
#[case("flag ? absent : selected", "const flag=false, selected='4px';")]
#[case("selected ?? absent", "const selected=0;")]
#[case("selected || absent", "const selected='4px';")]
#[case("selected && absent", "const selected=0;")]
#[case(
    "[flag ? selected : absent, e, f, '2px']",
    "const flag=true, selected='4px', e='8px', f='12px';"
)]
#[case(
    "[flag ? selected : absent, e, f, nested ? '2px' : '3px']",
    "const flag=true, nested=false, selected='4px', e='8px', f='12px';"
)]
#[case(
    "[flag ? absent : selected, e, f, nested ? '2px' : '3px']",
    "const flag=false, nested=true, selected='4px', e='8px', f='12px';"
)]
#[case(
    "flag ? [selected, '2px'] : [absent, '3px']",
    "const flag=true, selected='4px';"
)]
#[test]
#[serial]
fn unselected_branch_is_unread_when_it_is_in_the_tdz(
    #[case] expression: &str,
    #[case] setup: &str,
) {
    // Given: the unselected branch cannot be read before its lexical declaration.
    let compiled = lowered(expression);
    // When: the lowered assignment is evaluated in that authored scope.
    let actual = evaluate(&format!(
        "{setup} const node={compiled}; let absent; JSON.stringify('ok');"
    ));
    // Then: selection succeeds without reading that branch.
    assert_eq!(actual, "\"ok\"");
}

#[rstest]
#[case("reads.flag ? reads.a : reads.b")]
#[case("reads.a ?? reads.b")]
#[case("reads.a || reads.b")]
#[case("reads.zero && reads.b")]
#[case("({a:reads.a,b:reads.b})[index()]")]
#[case("[[reads.a,1,reads.c],[reads.d,reads.e,2]][index()]")]
#[case("[, [reads.d,reads.e,reads.f]][index()]")]
#[case("[[reads.a,reads.b,reads.c],[reads.d,reads.e,reads.f]][index()]")]
#[case("({0:[reads.a,reads.b,reads.c],1:[reads.d,reads.e,reads.f]})[index()]")]
#[case("[reads.a, reads.b, reads.c]")]
#[case("({b:reads.b,a:reads.a})[key]")]
#[case("({a:reads.a,a:reads.b})['a']")]
#[test]
#[serial]
fn reads_follow_source_when_selections_and_responsive_values_are_lowered(#[case] expression: &str) {
    // Given: getters and key coercion expose every read, including eager literal entries.
    let setup = "let trace=[]; const reads={}; for(const [key,value] of Object.entries({flag:true,a:'4px',b:'8px',c:'12px',d:'16px',e:'20px',f:'24px',zero:0})) Object.defineProperty(reads,key,{get(){trace.push(key);return value}}); const index=()=>{trace.push('index');return 1}; const key={toString(){trace.push('key');return 'a'}};";
    let expected = evaluate(&format!(
        "{setup} const value=({expression}); JSON.stringify(trace);"
    ));
    let compiled = lowered(expression);
    // When: the generated class and inline assignment are evaluated together.
    let actual = evaluate(&format!(
        "{setup} const node={compiled}; JSON.stringify(trace);"
    ));
    // Then: the exact authored read order and count survive.
    assert_eq!(actual, expected, "{expression}");
}

#[rstest]
#[case("state.choose ? state.a : state.b", "['4px']")]
#[case("state.a ?? state.b", "['4px']")]
#[case(
    "[state.choose ? state.a : state.b, state.c, state.d, '2px']",
    "['4px','12px','16px']"
)]
#[case("({a:state.a,b:state.b})[state.key]", "['8px']")]
#[case(
    "[[state.a,1,state.c],[state.d,state.e,2]][state.index]",
    "['16px','20px']"
)]
#[case("[, [state.d,state.e,state.f]][state.index]", "['16px','20px','24px']")]
#[case(
    "[[state.a,state.b,state.c],[state.d,state.e,state.f]][state.index]",
    "['16px','20px','24px']"
)]
#[case(
    "({0:[state.a,state.b,state.c],1:[state.d,state.e,state.f]})[state.index]",
    "['16px','20px','24px']"
)]
#[case("({a:state.a,a:state.b})['a']", "['8px']")]
#[case("({a:[state.a,state.b]})['a']", "['4px','8px']")]
#[test]
#[serial]
fn selected_values_survive_when_full_jsx_is_compiled(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: responsive and scalar selections with distinguishable branch values.
    let source = format!(
        "import {{Box}} from '@devup-ui/react'; function render(state){{return <Box gap={{{expression}}}/>}} const node=render({{choose:true,a:'4px',b:'8px',c:'12px',d:'16px',e:'20px',f:'24px',key:'b',index:1}}); JSON.stringify(Object.values(node.style));"
    );
    let compiled = compiled_jsx(&source);
    // When: the emitted JSX's actual attributes are evaluated.
    let actual = evaluate(&compiled);
    // Then: only values belonging to the selected assignment reach its variables.
    assert_eq!(
        actual,
        evaluate(&format!("JSON.stringify({expected})")),
        "{compiled}"
    );
}

#[test]
#[serial]
fn attribute_reads_stay_ordered_when_shorthand_and_children_share_capture() {
    // Given: source attributes and children with observable getters around a selected shorthand.
    let source = "import {Box} from '@devup-ui/react'; function render(state){return <Box id={state.before} px={state.choose ? state.a : state.b} title={state.after}>{state.child}</Box>} let trace=[]; const state={}; for(const [key,value] of Object.entries({before:'before',choose:true,a:'4px',b:'8px',after:'after',child:'child'})) Object.defineProperty(state,key,{get(){trace.push(key);return value}}); const node=render(state); JSON.stringify(trace);";
    let compiled = compiled_jsx(source);
    // When: all consumers and adjacent authored reads execute through the emitted expression.
    let actual = evaluate(&compiled);
    // Then: px reads its selected value once, between the adjacent attributes and before its child.
    assert_eq!(
        actual, "[\"before\",\"choose\",\"a\",\"after\",\"child\"]",
        "{compiled}"
    );
}

#[test]
#[serial]
fn yields_stay_in_the_generator_when_selected_values_are_captured() {
    // Given: lazy yielded alternatives and a later yielded attribute.
    let source = "import {Box} from '@devup-ui/react'; function* render(state){return <Box gap={state.flag ? yield state.a : yield absent} title={yield state.after}/>} const g=render({flag:true,a:'chosen',after:'later'}); const first=g.next(); const second=g.next('4px'); const last=g.next('title'); let absent; JSON.stringify([first.value,second.value,Object.values(last.value.style),last.value.title]);";
    let compiled = compiled_jsx(source);
    // When: the authored generator is resumed in order.
    let actual = evaluate(&compiled);
    // Then: yields remain in that scope and the unselected branch remains unread.
    assert_eq!(
        actual, "[\"chosen\",\"later\",[\"4px\"],\"title\"]",
        "{compiled}"
    );
}

#[test]
#[serial]
fn conditional_order_selects_matching_classes_when_assignment_values_are_shared() {
    // Given: both cascade orders consume the same responsive assignment.
    let source = "import {Box} from '@devup-ui/react';function render(state){return <Box styleOrder={state.order?1:2} gap={[state.a,state.b]}/>};const nodes=[render({order:true,a:'4px',b:'8px'}),render({order:false,a:'4px',b:'8px'})];";
    let output = extracted(source);
    let expected = [1, 2].map(|order| {
        let mut classes = output
            .styles
            .iter()
            .filter_map(|value| match value {
                crate::ExtractStyleValue::Dynamic(style) if style.style_order() == Some(order) => {
                    match value.extract(None) {
                        Some(crate::extract_style::style_property::StyleProperty::Variable {
                            class_name,
                            ..
                        }) => Some(class_name),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        classes.sort();
        assert_eq!(classes.len(), 2);
        classes
    });
    // When: each input condition selects its declared cascade order.
    let actual = evaluate(&format!(
        "{} JSON.stringify(nodes.map(node=>node.className.split(' ').filter(Boolean).sort()));",
        compiled_jsx(source)
    ));
    // Then: the selected classes agree with that input's emitted CSS order.
    assert_eq!(
        actual,
        serde_json::to_string(&expected).unwrap_or_else(|error| panic!("{error}"))
    );
}
