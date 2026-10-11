use super::{compile, reset};
use crate::ExtractStyleValue as Value;
use crate::assignment_test_support::{evaluate, jsx_js};
use crate::compiler_policy::tests::{associated, failed, input, offset};
use crate::extract_style::ProducerPolicy as Policy;
use crate::extract_style::compiler_receipts::ReceiptWitness;
use serial_test::serial;
pub(crate) fn names(receipts: &[ReceiptWitness]) -> Vec<String> {
    receipts
        .iter()
        .map(|receipt| receipt.produced.class().allocation.name.clone())
        .collect()
}

#[test]
#[serial]
fn lazy_reversed_demands_precede_sorted_materialization() {
    reset();
    let (_, _, witnesses) = compile(
        "A.tsx",
        "import {Box} from '@devup-ui/react'; const a = <Box color='red' background='blue'/>;",
    );
    let properties: Vec<_> = witnesses
        .iter()
        .take(2)
        .map(|witness| match &witness.operand {
            Value::Static(style) => style.property(),
            _ => panic!("static"),
        })
        .collect();
    assert_eq!(properties, ["color", "background"]);
    assert_eq!(
        crate::extract_style::compiler_receipts::PRODUCTIONS.get(),
        2
    );
}

#[test]
#[serial]
fn dynamic_whole_operand_runs_once_through_real_counter_entry() {
    reset();
    let source = "import {Box} from '@devup-ui/react'; let reads=0; const value=()=>{reads++;return 'red'}; const a=<Box color={value()}/>;";
    let (metadata, _, _) = compile("A.tsx", source);
    let result = evaluate(&format!(
        "{}\nJSON.stringify([reads,Object.values(a.style)]);",
        jsx_js(&metadata.code)
    ));
    assert_eq!(result, "[1,[\"red\"]]");
}

#[test]
#[serial]
fn preunion_global_aliases_complete_one_missing_root_demand_with_exact_receipt() {
    reset();
    let (_, styles, receipts, graph) = associated(input(
        "A.tsx",
        "import {globalCss} from '@devup-ui/react'; globalCss({body:{color:'red'}}); globalCss({body:{color:'red'}});",
    ));
    assert_eq!(styles.len(), 1);
    assert_eq!(receipts.len(), 1);
    assert_eq!(graph.operands.len(), 2);
    assert_eq!(graph.emissions.len(), 1);
    assert_eq!(graph.naming.len(), 1);
    assert_eq!(
        crate::extract_style::compiler_receipts::PRODUCTIONS.get(),
        1
    );
    assert_eq!(graph.emissions[0].receipt, receipts[0].id);
    assert!(
        graph
            .operands
            .iter()
            .all(|operand| operand.survivor == graph.emissions[0].survivor)
    );
    assert_eq!(graph.emissions[0].operand, styles[0]);
}
#[test]
#[serial]
fn styled_none_reuses_creation_context_and_reads_once_before_two_renders() {
    reset();
    let (metadata, _, receipts, graph) = associated(input(
        "A.tsx",
        "import {styled} from '@devup-ui/react'; const A=styled.div({width:state.size});",
    ));
    let actual = evaluate(&format!(
        "let reads=0; const state={{get size(){{reads++;return '13px'}}}}; {} const created=reads; const first=A({{}}); const second=A({{}}); JSON.stringify([created,reads,first.className===second.className]);",
        jsx_js(&metadata.code)
    ));
    assert_eq!(actual, "[1,1,true]");
    assert_eq!(receipts.len(), 1);
    assert!(
        graph
            .variables
            .iter()
            .any(|usage| usage.consumer.starts_with("__devupStyled")
                && usage.receipts == [receipts[0].id])
    );
    assert!(
        graph
            .variables
            .iter()
            .any(|usage| usage.route.is_none() && usage.receipts == [receipts[0].id])
    );
    assert_eq!(receipts[0].route.as_deref(), Some("A.tsx"));
}
#[test]
#[serial]
fn nested_public_current_shadows_counter_then_restores_root_names() {
    use crate::compiler_policy::tests::ObserveScope;
    reset();
    let nested = "import {Box} from '@devup-ui/react'; const a=<Box color='blue'/>;";
    let _ordinal = css::file_map::get_file_num_by_filename("A.tsx");
    let current = move || {
        crate::extract_without_source_map("nested.tsx", nested, Default::default())
            .unwrap_or_else(|error| panic!("{error}"))
    };
    let expected = current().code;
    reset();
    let _observer = ObserveScope::enter(move |retry| {
        assert!(!retry);
        let current = current();
        assert_eq!(
            current.code, expected,
            "public Current must keep its own names"
        );
        assert!(current.styles.iter().all(|value| matches!(value, Value::Static(style) if style.producer_policy() == Policy::Current)));
    });
    let (metadata, _, receipts, graph) = associated(input(
        "A.tsx",
        "import {globalCss,Box} from '@devup-ui/react'; globalCss({body:{color:'red'}}); const a=<Box bg='white'/>;",
    ));
    assert_eq!(receipts.len(), 2);
    assert!(receipts.iter().all(|receipt| matches!(&receipt.operand, Value::Static(style) if style.producer_policy() == Policy::CounterOriginal(0))));
    assert!(
        metadata
            .code
            .contains(&receipts[0].produced.class().allocation.name)
    );
    assert!(
        graph
            .invocations
            .iter()
            .all(|invocation| invocation.raw == "A.tsx")
    );
}
#[test]
#[serial]
fn supported_parser_surfaces_keep_generated_receipts_with_generate_and_skip_maps() {
    for code in [
        "import {Box} from '@devup-ui/react'; const a=<Box bg='red'/>;",
        "import {Box} from '@devup-ui/react'; const a=<Box color={value}/>;",
        "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{opacity:0},to:{opacity:1}});",
        "import {styled} from '@devup-ui/react'; const A=styled.div({color:'red'});",
        "import * as stylex from '@stylexjs/stylex'; const s=stylex.create({base:{color:'red'}}); const a=stylex.props(s.base);",
        "import {Box} from '@devup-ui/react'; const a=<Box className='p-4'/>;",
        "import {Box} from '@devup-ui/react'; const a=<Box typography='heading'/>;",
    ] {
        for mapped in [true, false] {
            reset();
            let mut source = input("matrix.tsx", code);
            source.source_map = mapped;
            let (metadata, styles, receipts, graph) = associated(source);
            assert_eq!(metadata.map.is_some(), mapped);
            let generated = styles
                .iter()
                .filter(|value| !matches!(value, Value::Typography(_)))
                .count();
            assert_eq!(graph.emissions.len(), generated);
            assert!(receipts.iter().all(|receipt| {
                metadata
                    .code
                    .contains(&receipt.produced.class().allocation.name)
            }));
        }
    }
}
#[test]
#[serial]
fn keyframes_keep_one_parent_receipt_without_member_demands() {
    reset();
    let (metadata, styles, witnesses) = compile(
        "A.tsx",
        "import {keyframes} from '@devup-ui/react'; const k=keyframes({from:{opacity:0},to:{opacity:1}});",
    );
    assert_eq!(styles.len(), 1);
    assert_eq!(
        crate::extract_style::compiler_receipts::PRODUCTIONS.get(),
        1
    );
    assert!(
        metadata
            .code
            .contains(&witnesses[0].produced.class().allocation.name)
    );
}

#[test]
#[serial]
fn auxiliary_generated_operands_report_real_markers_or_unavailable_positions() {
    use crate::compiler_policy::tests::ObserveScope;
    use crate::extract_style::{compiler_projection, compiler_receipts};
    for mapped in [true, false] {
        reset();
        let source = "import {Box} from '@devup-ui/react';\nconst a=<Box color={value}/>;";
        let at = offset(source, "value");
        let origin = crate::style_origin::actual("A.tsx", source, oxc_span::Span::new(at, at + 5))
            .unwrap_or_else(|| panic!("authored origin"));
        let expected = format!("A.tsx:{}:{}", origin.line, origin.column);
        let marker = serde_json::to_string(&origin).unwrap_or_else(|_| panic!("origin JSON"));
        let operand = if mapped {
            format!("{}({:?}, value)", crate::style_origin::MARKER, marker)
        } else {
            "value".into()
        };
        let _observer = ObserveScope::enter(move |retry| {
            assert!(!retry);
            let partial = format!(
                "import {{Box}} from '@devup-ui/react'; const a=<Box color={{{operand}}}/>;"
            );
            let Err(error) = crate::extract_class_map_from_code(
                "A.tsx",
                &partial,
                &crate::ExtractOption::default(),
                &rustc_hash::FxHashSet::default(),
                css::Naming::Own,
            ) else {
                panic!("auxiliary terminal")
            };
            assert!(error.is::<compiler_projection::ProjectionError>());
            if mapped {
                assert!(error.to_string().contains(&expected), "{error}");
            } else {
                assert!(
                    error.to_string().contains("position unavailable"),
                    "{error}"
                );
                assert!(!error.to_string().contains("A.tsx:1:1"), "{error}");
            }
            compiler_receipts::retain_terminal(error.as_ref());
        });
        let error = failed(input("A.tsx", source));
        assert!(error.is::<compiler_projection::ProjectionError>());
    }
}
