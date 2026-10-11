use super::*;
use crate::compiler_policy::tests::{associated, compile, input, names, reset};
use crate::extract_style::ProducerPolicy as Policy;
use serial_test::serial;
use std::collections::{BTreeMap, HashMap};

#[test]
#[serial]
fn parser_class_and_inline_projections_produce_once() {
    reset();
    let code = "import {Box} from '@devup-ui/react'; const a = <Box color={value} bg='red'/>;";
    let (metadata, styles, witnesses, graph) = associated(input("A.tsx", code));
    assert_eq!(PRODUCTIONS.get(), 2);
    assert_eq!(styles.len(), 2);
    assert_eq!(graph.naming.len(), 2, "root memo hits must not reacquire");
    let code = &metadata.code;
    for witness in witnesses {
        assert!(code.contains(&witness.produced.class().allocation.name));
        if let ProducedData::Dynamic(dynamic) = &witness.produced {
            assert!(code.contains(&dynamic.variable));
            assert_eq!(dynamic.variable_allocation, None);
        }
    }
}

#[test]
#[serial]
fn abandoned_retry_restores_maps_and_receipts_before_clean_continuation() {
    use crate::compiler_policy::tests::ObserveScope;
    reset();
    let retried = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = retried.clone();
    let mut injected = false;
    let _observer = ObserveScope::enter(move |retry| {
        if !retry && !injected {
            injected = true;
            crate::extract_class_map_from_code(
                "actual-aux.tsx",
                "import {css} from '@devup-ui/react'; const aux=css({color:'green'});",
                &Default::default(),
                &Default::default(),
                css::Naming::Own,
            )
            .unwrap_or_else(|error| panic!("{error}"));
            JOURNAL.with_borrow_mut(|journal| {
                let journal = journal.as_mut().unwrap_or_else(|| panic!("journal"));
                let operand = &journal.graph.operands[0];
                let receipt = journal
                    .witnesses
                    .iter()
                    .find(|receipt| {
                        super::super::compiler_request::same(&receipt.operand, &operand.operand)
                    })
                    .unwrap_or_else(|| panic!("real parsed receipt"));
                journal.graph.emissions.push(
                    crate::extract_style::compiler_associations::FinalAssociation {
                        operand: operand.operand.clone(),
                        survivor: operand.survivor,
                        receipt: receipt.id,
                    },
                );
                assert!(
                    !journal.graph.variables.is_empty()
                        && !journal.graph.rewrites.is_empty()
                        && !journal.graph.frames.is_empty()
                );
            });
        }
        if retry {
            observed.set(observed.get() + 1);
            assert_eq!(
                css::class_map::get_class_map(),
                std::collections::HashMap::new(),
                "abandoned class maps"
            );
            assert_eq!(
                css::file_map::get_file_map().len(),
                0,
                "abandoned delivery map"
            );
            assert_eq!(
                css::file_map::get_original_ids(),
                std::collections::BTreeMap::from([("A.tsx".into(), 0)])
            );
            JOURNAL.with_borrow(|journal| {
                let journal = journal
                    .as_ref()
                    .unwrap_or_else(|| panic!("Counter journal"));
                assert_eq!(journal.witnesses.len(), 0, "abandoned receipts");
                assert_eq!(journal.errors.len(), 0, "abandoned diagnostics");
                let graph = &journal.graph;
                assert_eq!(
                    [
                        graph.operands.len(),
                        graph.emissions.len(),
                        graph.naming.len(),
                        graph.frames.len(),
                        graph.variables.len(),
                        graph.rewrites.len(),
                        graph.orders.len()
                    ],
                    [0; 7],
                    "every abandoned graph table"
                );
                assert_eq!(graph.invocations.len(), 1, "abandoned auxiliary invocation");
                assert_eq!(graph.invocation, Some(graph.invocations[0].id));
                assert_eq!(graph.scope, None);
            });
        }
    });
    let source = "import {Box,css,styled} from '@devup-ui/react'; const pick=()=> 'red'; const a=<Box color={[value,other]} bg='blue'/>; const C=styled.div({width:size}); const s=css({color:pick()});";
    let (metadata, _, witnesses) = compile("A.tsx", source);
    assert_eq!(retried.get(), 1, "legitimate evaluable fallback must retry");
    let maps = css::class_map::get_class_map();
    let produced_names = names(&witnesses);
    drop(_observer);
    reset();
    let (computed, computed_edits, _, _) = crate::build_time_values::evaluate(
        source,
        "A.tsx",
        &crate::ExtractOption::default(),
        None,
        &Default::default(),
    )
    .unwrap_or_else(|| panic!("evaluable call"));
    assert_ne!(computed_edits.len(), 0);
    let (clean, _, clean_witnesses) = compile("A.tsx", &computed);
    assert_eq!(metadata.code, clean.code);
    assert_eq!(maps, css::class_map::get_class_map());
    assert_eq!(produced_names, names(&clean_witnesses));
}

#[test]
#[serial]
fn producer_identifier_is_immutable_and_rewrite_consumers_are_explicit() {
    reset();
    let (metadata, _, receipts, graph) = associated(input(
        "A.tsx",
        "import {Box} from '@devup-ui/react'; const a=<Box styleOrder={flag?1:2} color={[read(),other()]}/>;",
    ));
    let receipt = receipts.iter().find(|receipt| matches!(&receipt.produced, ProducedData::Dynamic(value) if value.identifier == "read()")).unwrap_or_else(|| panic!("real producer"));
    let ProducedData::Dynamic(produced) = &receipt.produced else {
        panic!("dynamic")
    };
    assert_eq!(produced.identifier, "read()");
    let usage = graph
        .variables
        .iter()
        .find(|usage| usage.consumer == "__devupValue" && usage.original.identifier() == "read()")
        .unwrap_or_else(|| panic!("sanctioned consumer"));
    assert_eq!(usage.original.identifier(), "read()");
    assert!(usage.receipts.contains(&receipt.id));
    assert_eq!(
        usage.receipts.len(),
        2,
        "primary and alternate orders share one actual use"
    );
    assert_eq!(usage.original.style_order(), None);
    assert!(graph.rewrites.iter().any(|edge| edge.scope == usage.scope
        && edge.before.identifier() == "read()"
        && edge.after.identifier() == "__devupValue"));
    assert!(metadata.code.contains("__devupValue"));
}
#[test]
#[serial]
fn discarded_child_keeps_full_collapsed_originals_without_completion() {
    reset();
    let _canonical = crate::compiler_policy::tests::ObserveScope::enter(|_| {});
    css::file_map::set_canonical_map(HashMap::from([
        ("child.css.ts".into(), "delivery.css.ts".into()),
        ("parent.css.ts".into(), "delivery.css.ts".into()),
    ]));
    let resolver = |_: &str, _: &str| {
        Some(crate::ResolvedModule { path: "child.css.ts".into(), code: "import {globalStyle} from '@devup-ui/react'; export const tone='red'; globalStyle('body',{ background: tone });".into() })
    };
    let mut source = input(
        "parent.css.ts",
        "import {tone} from './child.css'; import {globalStyle} from '@devup-ui/react'; globalStyle('body',{ background: tone });",
    );
    source.resolver = Some(&resolver);
    let (_, styles, receipts, graph) = associated(source);
    assert_eq!(styles.len(), 1);
    assert_eq!(receipts.len(), 1, "discarded global child must not reserve");
    assert_eq!(graph.emissions.len(), 1);
    let invocations = &graph.invocations;
    assert!(graph.operands.iter().any(|operand| matches!(&operand.operand, ExtractStyleValue::Static(style) if style.producer_policy() == Policy::CounterOriginal(1))));
    assert!(
        invocations
            .iter()
            .any(|invocation| invocation.raw == "child.css.ts" && invocation.original == 1)
    );
    assert!(
        graph
            .naming
            .iter()
            .all(|use_| invocations[use_.invocation.0].raw != "child.css.ts")
    );
    assert_eq!(invocations[receipts[0].invocation.0].raw, "parent.css.ts");
}
#[test]
#[serial]
fn repeated_keyframes_retain_ordered_members_raw_invocation_and_exact_survivor() {
    reset();
    let _canonical = crate::compiler_policy::tests::ObserveScope::enter(|_| {});
    let raw = "raw/../keys.tsx";
    css::file_map::set_original_ids(BTreeMap::from([(raw.into(), 7)]));
    css::file_map::set_canonical_map(HashMap::from([(raw.into(), "delivery.tsx".into())]));
    let (_, styles, receipts, graph) = associated(input(
        raw,
        "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{color:'red',opacity:0},to:{color:'blue',opacity:1}}); const b=keyframes({from:{color:'red',opacity:0},to:{color:'blue',opacity:1}});",
    ));
    assert_eq!(styles.len(), 1);
    assert_eq!(receipts.len(), 1);
    assert_eq!(graph.operands.len(), 2);
    assert_eq!(graph.emissions.len(), 1);
    assert_eq!(graph.invocations[0].raw, raw);
    assert_eq!(receipts[0].route.as_deref(), Some("delivery.tsx"));
    for operand in &graph.operands {
        let ExtractStyleValue::Keyframes(frames) = &operand.operand else {
            panic!("keyframes")
        };
        assert_eq!(frames.producer_policy(), Policy::CounterOriginal(7));
        let members: Vec<_> = frames
            .keyframes
            .values()
            .flatten()
            .map(crate::extract_style::extract_static_style::ExtractStaticStyle::property)
            .collect();
        assert_eq!(members, ["color", "opacity", "color", "opacity"]);
        assert!(
            frames
                .keyframes
                .values()
                .flatten()
                .all(|member| member.producer_policy() == Policy::CounterOriginal(7))
        );
        assert_eq!(operand.survivor, graph.emissions[0].survivor);
    }
    assert_eq!(graph.emissions[0].receipt, receipts[0].id);
}
