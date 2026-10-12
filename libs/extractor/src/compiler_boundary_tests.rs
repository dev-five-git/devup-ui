use super::{ObserveScope, associated, failed, input, one, reset};
use crate::compiler_policy::{CounterCompileError, with_counter_extract};
use crate::extract_style::{
    compiler_projection as projection, compiler_receipts as receipts,
    extract_style_value::ExtractStyleValue as Value,
};
use projection::ProjectionError as Fault;
use receipts::coverage_tests::maps;
use rstest::rstest;
use serial_test::serial;
use std::cell::Cell;

const SOURCE: &str = "import {Box,globalCss} from '@devup-ui/react'; const a=<Box color={value} bg={other} typography='heading'/>; globalCss`body{margin:0;}`; globalCss({body:{opacity:1},imports:['a.css'],fontFaces:[{fontFamily:'X',src:'x.woff'}]});";

fn journal<T>(action: impl FnOnce(&mut receipts::Journal) -> T) -> T {
    receipts::JOURNAL.with_borrow_mut(|journal| action(one(journal.as_mut())))
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn current_parser_operands_are_rejected_by_counter_consumers(#[case] variable: bool) {
    // Given: Current operands are acquired by the public parser before Counter starts.
    reset();
    let current = crate::extract_without_source_map("current.tsx", SOURCE, Default::default())
        .unwrap_or_else(|error| panic!("{error}"));
    let before = maps();
    let _observer = ObserveScope::enter(move |retry| {
        assert!(!retry);
        let operand = one(current
            .styles
            .iter()
            .filter(|value| matches!(value, Value::Dynamic(style) if style.property() == "color")));
        let observed = maps();
        assert!(matches!(
            projection::context(operand, Some("boundary.tsx")),
            Err(Fault::Producer(
                crate::extract_style::CounterProducerError::WrongPolicy
            ))
        ));
        // When: the active Counter consumer receives a real Current operand.
        let projected = if variable {
            projection::inline(operand, Some("boundary.tsx")).map(|value| value.variable_name)
        } else {
            crate::gen_class_name::class_name(operand, Some("boundary.tsx"))
        };
        assert_eq!(projected, None);
        assert_eq!(maps(), observed);
    });
    let error = failed(input("boundary.tsx", SOURCE));
    // Then: the typed terminal rejects publication and all four maps roll back.
    assert!(
        matches!(error.downcast_ref(), Some(Fault::Terminal(message)) if message.contains("counter producer requires retained CounterOriginal policy"))
    );
    assert_eq!(maps(), before);
    drop(_observer);
    let (_, _, witnesses, _) = associated(input("boundary.tsx", SOURCE));
    assert_eq!(witnesses.len(), 3);
}

#[test]
#[serial]
fn authored_nonallocating_operands_reject_allocation_without_inventing_demands() {
    // Given: Typography, CSS, Import and FontFace are real authored parser outputs.
    reset();
    let before = maps();
    let _observer = ObserveScope::enter(|retry| {
        assert!(!retry);
        let operands = journal(|journal| {
            journal
                .graph
                .operands
                .iter()
                .map(|row| row.operand.clone())
                .collect::<Vec<_>>()
        });
        let mut checked = 0;
        for operand in operands {
            match operand {
                Value::Typography(_) | Value::Css(_) | Value::Import(_) | Value::FontFace(_) => {
                    let observed = maps();
                    // When: allocation protocol is fault-injected, not a naming demand.
                    let result = projection::produce(&operand, Some("boundary.tsx"));
                    assert!(matches!(
                        result,
                        Err(Fault::Producer(
                            crate::extract_style::CounterProducerError::WrongPolicy
                        ))
                    ));
                    let result = projection::context(&operand, Some("boundary.tsx"));
                    assert!(matches!(
                        result,
                        Err(Fault::Producer(
                            crate::extract_style::CounterProducerError::WrongPolicy
                        ))
                    ));
                    assert_eq!(maps(), observed);
                    assert_eq!(receipts::collect_operand(result, &operand), None);
                    checked += 1;
                }
                Value::Static(_) | Value::Dynamic(_) | Value::Keyframes(_) => {}
            }
        }
        assert_eq!(checked, 4);
    });
    let error = failed(input("boundary.tsx", SOURCE));
    // Then: every no-origin operand is transported, with exact rollback.
    assert!(
        matches!(error.downcast_ref(), Some(Fault::Terminal(message)) if message.lines().count() == 4)
    );
    assert_eq!(maps(), before);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[serial]
fn no_site_variable_uses_actual_acquired_receipts_or_rejects_ambiguity(#[case] mode: u8) {
    // Given: remove only the site from actual parsed Counter dynamic operands.
    reset();
    let before = maps();
    let _observer = ObserveScope::enter(move |retry| {
        assert!(!retry);
        let mut values = journal(|journal| {
            journal
                .graph
                .operands
                .iter()
                .filter_map(|row| match &row.operand {
                    Value::Dynamic(style) => Some(style.clone()),
                    Value::Static(_)
                    | Value::Typography(_)
                    | Value::Keyframes(_)
                    | Value::Css(_)
                    | Value::Import(_)
                    | Value::FontFace(_) => None,
                })
                .collect::<Vec<_>>()
        });
        assert_eq!(values.len(), 2);
        for style in &mut values {
            assert!(style.site().is_some());
            style.site = None;
        }
        let operand = Value::Dynamic(values[0].clone());
        let expected = match mode {
            0 => None,
            1..=4 => {
                // These are explicitly injected no-site requests, never claimed authored.
                assert!(
                    crate::gen_class_name::class_name(&operand, Some("boundary.tsx")).is_some()
                );
                let produced = journal(|journal| one(journal.witnesses.last()).produced.clone());
                let receipts::ProducedData::Dynamic(produced) = produced else {
                    panic!("dynamic production")
                };
                assert_eq!(produced.site, None);
                assert_eq!(produced.identifier, values[0].identifier());
                assert!(produced.variable_allocation.is_some());
                let expected = produced.variable.clone();
                if mode == 2 {
                    assert!(
                        crate::gen_class_name::class_name(&operand, Some("second.tsx")).is_some()
                    );
                }
                if mode == 3 || mode == 4 {
                    let other = match mode {
                        3 => Value::Dynamic(values[1].clone()),
                        4 => journal(|journal| {
                            one(journal
                                .graph
                                .operands
                                .iter()
                                .filter(|row| matches!(row.operand, Value::Static(_))))
                            .operand
                            .clone()
                        }),
                        _ => panic!("association fault mode"),
                    };
                    assert!(
                        crate::gen_class_name::class_name(&other, Some("boundary.tsx")).is_some()
                    );
                    journal(|journal| {
                        let id = one(journal.witnesses.last()).id;
                        let edge = one(journal.graph.orders.last_mut());
                        edge.before = operand.clone();
                        assert_eq!(edge.receipt, id);
                    });
                }
                Some(expected)
            }
            _ => panic!("test mode"),
        };
        let observed = maps();
        // When: the actual inline extractor consumer resolves the injected site loss.
        let actual = projection::inline(&operand, Some("boundary.tsx"));
        match mode {
            0 | 3 | 4 => assert!(actual.is_none()),
            1 | 2 => {
                let actual = one(actual);
                assert_eq!(Some(actual.variable_name), expected);
                assert_eq!(
                    actual.identifier,
                    values[0].conversion().inline(values[0].identifier())
                );
                journal(|journal| {
                    let usage = one(journal.graph.variables.last());
                    assert_eq!(usage.original.identifier(), values[0].identifier());
                    assert_eq!(usage.consumer, values[0].identifier());
                    assert_ne!(usage.receipts.len(), 0);
                    assert!(usage.receipts.iter().all(|id| matches!(&journal.witnesses[id.0].produced, receipts::ProducedData::Dynamic(produced) if produced.identifier == values[0].identifier() && produced.site.is_none())));
                });
            }
            _ => panic!("test mode"),
        }
        assert_eq!(maps(), observed);
    });
    let published = Cell::new(false);
    let result = with_counter_extract(input("boundary.tsx", SOURCE), |_, batch| {
        published.set(true);
        batch.consume(|view| {
            assert_eq!(view.associations.emissions.len(), 3);
            Err::<(), _>(37)
        })
    });
    // Then: either a typed terminal or exact consumer rejection rolls back everything.
    match mode {
        0 | 3 | 4 => {
            assert!(!published.get());
            let error = match one(result.err()) {
                CounterCompileError::Compile(error) => error,
                CounterCompileError::Consumer(error) => panic!("unexpected consumer {error}"),
            };
            assert!(
                matches!(error.downcast_ref(), Some(Fault::Terminal(message)) if message.contains("counter variable requires an unambiguous finalized receipt"))
            );
        }
        1 | 2 => {
            assert!(published.get());
            assert!(matches!(result, Err(CounterCompileError::Consumer(37))));
        }
        _ => panic!("test mode"),
    }
    assert_eq!(maps(), before);
    drop(_observer);
    let (_, _, witnesses, _) = associated(input("boundary.tsx", SOURCE));
    assert_eq!(witnesses.len(), 3);
}

#[test]
#[serial]
fn rollback_allows_clean_real_parser_retry_after_boundary_rejection() {
    // Given: a real consumed batch rejection, with all naming state restored.
    reset();
    let before = maps();
    let result = with_counter_extract(input("boundary.tsx", SOURCE), |_, batch| {
        batch.consume(|_| Err::<(), _>(37))
    });
    assert!(matches!(result, Err(CounterCompileError::Consumer(37))));
    assert_eq!(maps(), before);
    // When: the identical real source is compiled again without injection.
    let (metadata, _, witnesses, graph) = associated(input("boundary.tsx", SOURCE));
    // Then: the true D9 original and all rendered class/variable receipts agree.
    assert_eq!(css::file_map::get_original_ids()["boundary.tsx"], 0);
    assert_eq!(witnesses.len(), 3);
    assert_eq!(graph.emissions.len(), 3);
    for witness in witnesses {
        assert_eq!(witness.produced.class().original, 0);
        assert!(
            metadata
                .code
                .contains(&witness.produced.class().allocation.name)
        );
    }
}
