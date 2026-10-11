use crate::compiler_policy::tests::{ObserveScope, associated, input, reset};
use crate::compiler_policy::{CounterCompileError, with_counter_extract};
use crate::extract_style::{
    compiler_associations as associations, compiler_projection as projection,
    compiler_receipts as receipts, extract_style_value::ExtractStyleValue as Value,
};
use rstest::rstest;
use serial_test::serial;
use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    convert::Infallible,
};

pub(crate) fn failed(
    input: crate::compiler_policy::CompilerInput<'_>,
) -> Box<dyn std::error::Error> {
    let mut errors =
        Vec::from_iter(with_counter_extract(input, |_, _| Ok::<_, Infallible>(())).err());
    assert_eq!(errors.len(), 1);
    match errors.remove(0) {
        CounterCompileError::Compile(error) => error,
        CounterCompileError::Consumer(error) => match error {},
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Maps {
    classes: HashMap<String, HashMap<String, usize>>,
    files: Vec<(String, usize)>,
    originals: BTreeMap<String, u32>,
    canonical: HashMap<String, String>,
}
fn maps() -> Maps {
    let mut files: Vec<_> = css::file_map::get_file_map().into_iter().collect();
    files.sort();
    Maps {
        classes: css::class_map::get_class_map(),
        files,
        originals: css::file_map::get_original_ids(),
        canonical: css::file_map::get_canonical_map(),
    }
}
fn operands() -> Vec<Value> {
    receipts::JOURNAL.with_borrow(|journal| {
        journal
            .iter()
            .flat_map(|journal| &journal.graph.operands)
            .map(|row| row.operand.clone())
            .collect()
    })
}

#[rstest]
#[case(
    "import * as stylex from '@stylexjs/stylex'; const k=stylex.keyframes({from:{'opacity':0}});"
)]
#[case("import {keyframes} from '@devup-ui/react'; const k=keyframes({from:{'opacity':0}});")]
#[case("import {keyframes} from '@devup-ui/react'; const k=keyframes`from{opacity:0;}`;")]
#[case(
    "import * as stylex from '@stylexjs/stylex'; const s=stylex.create({base:(height)=>({'height':height})}); const a=stylex.props(s.base(value));"
)]
#[serial]
fn parser_rejects_configuration_corruption_when_projection_demands_a_name(#[case] body: &str) {
    // Given: a real resolver crosses the compilation boundary before naming.
    reset();
    let before = maps();
    let resolver = |_: &str, _: &str| {
        receipts::JOURNAL.with_borrow_mut(|journal| {
            journal.iter_mut().for_each(|journal| {
                journal.config.prefix.push_str("corrupt");
            });
        });
        Some(crate::ResolvedModule {
            path: "constant.ts".into(),
            code: "export const v=1;".into(),
        })
    };
    let source = format!(
        "import {{v}} from './constant'; import {{css as probeCss}} from '@devup-ui/react'; const probe=probeCss({{opacity:v}}); {body}"
    );
    let mut request = input("coverage.tsx", &source);
    request.resolver = Some(&resolver);
    // When: the actual visitor attempts its class/keyframe/early-variable projection.
    let error = failed(request);
    // Then: typed, located compilation failure rolls every naming map back.
    assert!(
        matches!(error.downcast_ref::<projection::ProjectionError>(), Some(projection::ProjectionError::Terminal(message)) if message.contains("coverage.tsx:") && message.contains("counter naming configuration changed")),
        "{error}"
    );
    assert_eq!(maps(), before);
}

#[test]
#[serial]
fn sealing_rejects_a_real_variable_when_its_finalized_association_is_lost() {
    // Given: parsed variable uses, not a fabricated journal or batch.
    reset();
    let before = maps();
    let _observer = ObserveScope::enter(|retry| {
        assert!(!retry);
        receipts::JOURNAL.with_borrow_mut(|journal| {
            assert!(journal.is_some());
            journal.iter_mut().for_each(|journal| {
                let graph = &mut journal.graph;
                assert_eq!(graph.variables.len(), 1);
                assert_eq!(graph.variables[0].receipts.len(), 1);
                graph.variables[0].receipts.clear();
            });
        });
    });
    let published = Cell::new(false);
    // When: sealing sees the damaged real association after successful parsing.
    let result = with_counter_extract(
        input(
            "coverage.tsx",
            "import {Box} from '@devup-ui/react'; const a=<Box color={value}/>;",
        ),
        |_, batch| {
            published.set(true);
            batch.consume(|_| Ok::<_, ()>(()))
        },
    );
    // Then: conversion preserves the Variable variant and the consumer never runs.
    assert!(
        matches!(result, Err(CounterCompileError::Compile(ref error)) if matches!(error.downcast_ref::<projection::ProjectionError>(), Some(projection::ProjectionError::Variable)))
    );
    assert!(!published.get());
    assert_eq!(maps(), before);
}

#[test]
#[serial]
fn consumed_parser_batch_preserves_consumer_error_and_rolls_back_all_maps() {
    // Given: a real batch containing static and dynamic finalized productions.
    reset();
    let before = maps();
    // When: the sealed-view consumer rejects a nonempty batch.
    let result = with_counter_extract(
        input(
            "coverage.tsx",
            "import {Box} from '@devup-ui/react'; const a=<Box color={value} bg='red'/>;",
        ),
        |metadata, batch| {
            batch.consume(|view| {
                assert_eq!(view.styles.len(), 2);
                assert_eq!(view.witnesses.len(), 2);
                assert_eq!(view.associations.emissions.len(), 2);
                assert!(view.styles.windows(2).all(|pair| pair[0] <= pair[1]));
                for witness in view.witnesses {
                    assert!(
                        metadata
                            .code
                            .contains(&witness.produced.class().allocation.name)
                    );
                }
                Err::<(), _>(37)
            })
        },
    );
    // Then: the exact consumer value, not a Compile error, crosses the API.
    assert!(matches!(result, Err(CounterCompileError::Consumer(37))));
    assert_eq!(maps(), before);
}

#[test]
#[serial]
fn collection_retains_real_parser_operands_when_duplicates_collapse() {
    // Given: the observer borrows actual pre-union operands in the active invocation.
    reset();
    let _observer = ObserveScope::enter(|retry| {
        assert!(!retry);
        let operands = operands();
        assert_eq!(operands.len(), 2);
        let mut collected = associations::CollectedStyles::default();
        // When: both genuine duplicates enter through the production extend inlet.
        collected.extend(operands);
        // Then: one survivor retains both original occurrence associations.
        assert_eq!(collected.len(), 1);
        receipts::JOURNAL.with_borrow(|journal| {
            assert!(journal.is_some());
            journal.iter().for_each(|journal| {
                let rows = &journal.graph.operands;
                assert_eq!(rows.len(), 4);
                assert_eq!(rows[2].survivor, rows[3].survivor);
                assert_eq!(rows[2].invocation, rows[0].invocation);
            });
        });
    });
    let (_, styles, _, _) = associated(input(
        "coverage.tsx",
        "import {css} from '@devup-ui/react'; const a=css({color:'red'}); const b=css({color:'red'});",
    ));
    assert_eq!(styles.len(), 1);
}

#[test]
#[serial]
fn operand_diagnostics_keep_location_when_real_parser_operands_are_transport_errors() {
    // Given: static, dynamic, keyframe and raw-CSS operands from the real visitor.
    reset();
    let before = maps();
    let _observer = ObserveScope::enter(|retry| {
        assert!(!retry);
        let operands = operands();
        assert_eq!(operands.len(), 6);
        assert_eq!(receipts::collect(Ok(operands.len()), None), Some(6));
        // When: the projection protocol transports a failure for each actual operand.
        for operand in operands {
            if matches!(
                operand,
                Value::Css(_) | Value::Import(_) | Value::FontFace(_)
            ) {
                let before = maps();
                assert_eq!(
                    crate::gen_class_name::class_name(&operand, Some("coverage.tsx")),
                    None
                );
                assert_eq!(maps(), before);
            }
            assert_eq!(receipts::collect_operand(Ok(17), &operand), Some(17));
            assert_eq!(
                receipts::collect_operand::<()>(
                    Err(projection::ProjectionError::Context),
                    &operand
                ),
                None
            );
        }
    });
    let error = failed(input(
        "coverage.tsx",
        "import {Box,keyframes,globalCss} from '@devup-ui/react'; const a=<Box color='red' bg={value}/>; const k=keyframes({from:{opacity:0}}); globalCss`body{margin:0;}`; globalCss({imports:['a.css'],fontFaces:[{fontFamily:'X',src:'x.woff'}]});",
    ));
    // Then: three source-located operands and truthful demand transport survive erasure.
    let message = error.to_string();
    assert_eq!(message.lines().count(), 6);
    assert_eq!(message.matches("coverage.tsx:").count(), 6, "{message}");
    assert_eq!(
        message
            .lines()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "{message}"
    );
    assert_eq!(maps(), before);
}

#[test]
fn retry_error_formats_as_a_continuation_when_erased_for_transport() {
    // Given: actual parser/evaluator output requiring a clean continuation.
    let values = crate::build_time_values::evaluate(
        "import {css} from '@devup-ui/react'; const pick=()=>1; const s=css({opacity:pick()});",
        "coverage.tsx",
        &Default::default(),
        None,
        &Default::default(),
    );
    assert!(values.is_some());
    values.into_iter().for_each(|values| {
        assert_ne!(values.1.len(), 0);
        let error = crate::compiler_policy::retry(values, &[]);
        // When: an error consumer renders the erased diagnostic.
        let diagnostic = error.to_string();
        // Then: transport describes deferred evaluation, not a terminal naming failure.
        assert_eq!(diagnostic, "deferred Counter evaluation continuation");
    });
}
