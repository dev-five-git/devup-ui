use oxc_ast::{
    AstKind,
    ast::{Expression, PropertyKey},
};
use oxc_span::{GetSpan, Span};
use oxc_syntax::GetNodeId;
use rstest::rstest;

use super::parsed;
use crate::imported_constants::consumer::{GuardKind, ReadPlan, plan};

fn observations<'a>(
    result: &ReadPlan,
    source: &'a str,
) -> Vec<(Span, Vec<(&'a str, &'static str)>)> {
    result
        .guards
        .iter()
        .map(|(slot, guards)| {
            (
                *slot,
                guards
                    .iter()
                    .map(|guard| {
                        (
                            guard.test.source_text(source),
                            match guard.kind {
                                GuardKind::Truthy => "truthy",
                                GuardKind::Falsy => "falsy",
                                GuardKind::Nullish => "nullish",
                            },
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn shorthand_when_nodes_share_spans_keeps_branch_slots_and_outer_first_guards() {
    // Given
    let source = "import {css} from '@devup-ui/react';import {color} from './data';const outer=true,inner=false;if(outer){if(inner){css({color})}else{css({color})}}";
    parsed(source, |program, semantic| {
        let properties: Vec<_> = semantic
            .nodes()
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::ObjectProperty(property) if property.shorthand => Some(property),
                _ => None,
            })
            .collect();
        assert_eq!(properties.len(), 2);
        let mut slots = Vec::new();
        for property in properties {
            let PropertyKey::StaticIdentifier(key) = &property.key else {
                panic!("shorthand key must be a static identifier");
            };
            let Expression::Identifier(reference) = &property.value else {
                panic!("shorthand value must be an identifier reference");
            };
            assert_eq!(property.span, key.span);
            assert_eq!(key.span, reference.span);
            assert_eq!(reference.span.source_text(source), "color");
            let property_id = GetNodeId::node_id(property);
            let key_id = GetNodeId::node_id(key.as_ref());
            let reference_id = GetNodeId::node_id(reference.as_ref());
            assert_ne!(property_id, key_id);
            assert_ne!(property_id, reference_id);
            assert_ne!(key_id, reference_id);
            slots.push(reference.span);
        }
        slots.sort_by_key(|span| span.start);
        // When
        let result = plan(program, semantic, &crate::ExtractOption::default());
        // Then
        assert_eq!(result.slots, slots);
        assert_eq!(
            observations(&result, source),
            [
                (slots[0], vec![("outer", "truthy"), ("inner", "truthy")]),
                (slots[1], vec![("outer", "truthy"), ("inner", "falsy")]),
            ]
        );
        assert_eq!(result.failures.len(), 0);
    });
}

#[test]
fn call_when_nested_in_syntax_wrappers_keeps_one_slot_and_outer_first_guards() {
    // Given
    let source = "import {Box} from '@devup-ui/react';import {read} from './data';const outer=true,inner=false;if(outer){inner || <Box color={((read() as string)!)}/>;}";
    parsed(source, |program, semantic| {
        let call = semantic
            .nodes()
            .iter()
            .find_map(|node| match node.kind() {
                AstKind::CallExpression(call) if call.span.source_text(source) == "read()" => {
                    Some(call)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("authored read call must be in the full semantic tree"));
        let wrappers: Vec<_> = semantic
            .nodes()
            .ancestor_kinds(GetNodeId::node_id(call))
            .filter_map(|ancestor| match ancestor {
                AstKind::ParenthesizedExpression(_)
                | AstKind::TSAsExpression(_)
                | AstKind::TSNonNullExpression(_) => Some(ancestor.span().source_text(source)),
                _ => None,
            })
            .collect();
        assert_eq!(
            wrappers,
            [
                "read() as string",
                "(read() as string)",
                "(read() as string)!",
                "((read() as string)!)",
            ]
        );
        // When
        let result = plan(program, semantic, &crate::ExtractOption::default());
        // Then
        assert_eq!(result.slots, [call.span]);
        assert_eq!(
            observations(&result, source),
            [(call.span, vec![("outer", "truthy"), ("inner", "falsy")]),]
        );
        assert_eq!(result.failures.len(), 0);
    });
}

#[rstest]
#[case("enabled?read():other()")]
#[case("enabled||read()")]
fn whole_slot_when_conditional_or_logical_keeps_no_self_guard(#[case] expression: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';import {{read,other}} from './data';const enabled=true;css({{color:{expression}}});"
    );
    parsed(&source, |program, semantic| {
        let slot = semantic
            .nodes()
            .iter()
            .find_map(|node| match node.kind() {
                AstKind::ConditionalExpression(_) | AstKind::LogicalExpression(_)
                    if node.kind().span().source_text(&source) == expression =>
                {
                    Some(node.kind().span())
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!("authored whole expression must be in the full semantic tree")
            });
        // When
        let result = plan(program, semantic, &crate::ExtractOption::default());
        // Then
        assert_eq!(result.slots, [slot]);
        assert_eq!(observations(&result, &source), [(slot, vec![])]);
        assert_eq!(result.failures.len(), 0);
    });
}
