use super::{associated, input, one, reset};
use crate::ExtractStyleValue as Value;
use crate::assignment_test_support::{evaluate, jsx_js};
use crate::extract_style::ProducerPolicy;
use crate::extract_style::compiler_receipts::{PRODUCTIONS, ProducedData, WitnessDisposition};
use crate::extract_style::compiler_request::same;
use serial_test::serial;

#[test]
#[serial]
fn early_stylex_variable_keeps_its_receipt_when_unrelated_css_demands_a_name() {
    css::admission::with_admission(|| {
        // Given: a parameter-backed StyleX use followed by an unrelated static demand.
        reset();
        let filename = "association.tsx";
        let source = "import * as stylex from '@stylexjs/stylex'; import {css} from '@devup-ui/react'; const styles=stylex.create({base:(tone)=>({'color':tone})}); const node=stylex.props(styles.base('red')); const unrelated=css({color:'blue'});";

        // When: the actual Counter parser seals and publishes its real associations.
        let (metadata, styles, witnesses, graph) = associated(input(filename, source));

        // Then: both demands emit, but only the dynamic witness belongs to the variable.
        assert_eq!(PRODUCTIONS.get(), 2);
        assert_eq!(styles.len(), 2);
        assert_eq!(witnesses.len(), 2);
        assert_ne!(witnesses[0].id, witnesses[1].id);
        let usage = one(graph.variables.iter());
        assert_eq!(usage.consumer, "tone");
        assert_eq!(usage.original.identifier(), "tone");
        assert_eq!(usage.original.property(), "color");
        assert_eq!(
            usage.original.producer_policy(),
            ProducerPolicy::CounterOriginal(0)
        );
        assert_eq!(usage.scope, None);
        assert_eq!(usage.route, None);
        assert_eq!(graph.frames.len(), 0);
        assert_eq!(graph.rewrites.len(), 0);
        let invocation = one(graph.invocations.iter());
        assert_eq!(usage.invocation, invocation.id);
        assert_eq!(invocation.raw, filename);
        assert_eq!(invocation.original, 0);
        let site = one(usage.original.site());
        assert_eq!(site.file, css::sparse_site::SourceFile::D9(0));
        assert_eq!(
            site.at,
            source
                .find("tone})")
                .unwrap_or_else(|| panic!("authored value token"))
        );
        assert_eq!(site.role, 0);

        let dynamic = one(witnesses.iter().filter(|witness| {
            matches!(&witness.operand, Value::Dynamic(style) if style.identifier() == "tone")
        }));
        let unrelated = one(witnesses.iter().filter(|witness| {
            matches!(&witness.operand, Value::Static(style) if style.property() == "color" && style.value() == "blue")
        }));
        let ProducedData::Dynamic(produced) = &dynamic.produced else {
            panic!("real dynamic production")
        };
        assert!(matches!(&unrelated.produced, ProducedData::Allocation(_)));
        assert_eq!(produced.identifier, "tone");
        assert_eq!(produced.variable, site.variable_name(&usage.config.prefix));
        assert!(same(
            &dynamic.operand,
            &Value::Dynamic(usage.original.clone())
        ));
        assert_eq!(usage.receipts, [dynamic.id]);
        assert!(!usage.receipts.contains(&unrelated.id));

        assert_eq!(graph.orders.len(), 2);
        let dynamic_order = one(graph
            .orders
            .iter()
            .filter(|edge| edge.receipt == dynamic.id));
        let unrelated_order = one(graph
            .orders
            .iter()
            .filter(|edge| edge.receipt == unrelated.id));
        assert_eq!(graph.orders[0].receipt, dynamic.id);
        assert_eq!(graph.orders[1].receipt, unrelated.id);
        let original = Value::Dynamic(usage.original.clone());
        assert!(same(&dynamic_order.before, &original));
        assert!(same(&dynamic_order.after, &original));
        assert!(!same(&unrelated_order.before, &original));
        assert!(!same(&unrelated_order.after, &original));
        for edge in &graph.orders {
            assert_eq!(edge.invocation, usage.invocation);
            assert_eq!(edge.scope, usage.scope);
        }

        assert_eq!(graph.operands.len(), 2);
        assert_eq!(graph.emissions.len(), 2);
        for witness in &witnesses {
            assert_eq!(witness.invocation, invocation.id);
            assert_eq!(witness.disposition, WitnessDisposition::Emission);
            assert_eq!(witness.route.as_deref(), Some(filename));
            assert_eq!(
                styles
                    .iter()
                    .filter(|value| same(value, &witness.operand))
                    .count(),
                1
            );
            let emission = one(graph
                .emissions
                .iter()
                .filter(|edge| edge.receipt == witness.id));
            assert!(same(&emission.operand, &witness.operand));
            let operand = one(graph
                .operands
                .iter()
                .filter(|row| row.survivor == emission.survivor));
            assert!(same(&operand.operand, &witness.operand));
            assert_eq!(operand.invocation, invocation.id);
            let naming = one(graph.naming.iter().filter(|row| row.receipt == witness.id));
            assert!(same(&naming.operand, &witness.operand));
            assert_eq!(naming.invocation, invocation.id);
            assert_eq!(naming.scope, None);
            assert_eq!(naming.route.as_deref(), Some(filename));
        }

        let actual = evaluate(&format!(
            "{} JSON.stringify([node.className,Object.keys(node.style),Object.values(node.style),unrelated]);",
            jsx_js(&metadata.code)
        ));
        let expected = serde_json::json!([
            dynamic.produced.class().allocation.name,
            [produced.variable],
            ["red"],
            unrelated.produced.class().allocation.name
        ]);
        assert_eq!(actual, expected.to_string());
    });
}
