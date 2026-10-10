use super::{dynamic, expression};
use crate::{ExtractStyleProp, ExtractStyleValue, provenance::join_naming};
use css::Naming;
use oxc_allocator::Allocator;
use rstest::rstest;
use std::collections::BTreeMap;

#[rstest]
#[case("conditional")]
#[case("enum")]
#[case("member")]
#[case("expression")]
fn imported_provenance_reaches_every_nested_consumer(#[case] shape: &str) {
    // Given: mixed preexisting ownership; joining Own must not erase Risky.
    let allocator = Allocator::default();
    let source = expression(&allocator, "input");
    let mut already_risky = dynamic("color", "first");
    already_risky.join_naming(Naming::Risky);
    let values = vec![already_risky, dynamic("background-color", "second")];
    let leaf = || {
        ExtractStyleProp::StaticArray(
            values
                .iter()
                .cloned()
                .map(ExtractStyleProp::Static)
                .collect(),
        )
    };
    let prop = match shape {
        "conditional" => ExtractStyleProp::Conditional {
            condition: source,
            consequent: Some(Box::new(leaf())),
            alternate: Some(Box::new(leaf())),
        },
        "enum" => ExtractStyleProp::Enum {
            condition: source,
            map: BTreeMap::from([("a".into(), vec![leaf()]), ("b".into(), vec![leaf()])]),
        },
        "member" => ExtractStyleProp::MemberExpression {
            expression: source,
            map: BTreeMap::from([
                ("a".into(), Box::new(leaf())),
                ("b".into(), Box::new(leaf())),
            ]),
        },
        "expression" => ExtractStyleProp::Expression {
            expression: source,
            styles: values,
        },
        _ => panic!("fixture shape"),
    };
    let mut props = [prop];
    // When
    join_naming(&mut props, Naming::Risky);
    join_naming(&mut props, Naming::Own);
    // Then: both branches/entries retain every property and all become Risky.
    let actual: Vec<_> = props[0]
        .extract()
        .into_iter()
        .map(|value| match value {
            ExtractStyleValue::Dynamic(style) => (style.property().to_string(), style.naming()),
            other => panic!("unexpected record {other:?}"),
        })
        .collect();
    let pair = vec![
        ("color".to_string(), Naming::Risky),
        ("background-color".to_string(), Naming::Risky),
    ];
    let expected = if shape == "expression" {
        pair
    } else {
        pair.iter().chain(&pair).cloned().collect()
    };
    assert_eq!(actual, expected);
}

#[test]
fn typography_identity_is_unchanged_when_dependency_naming_is_joined() {
    // Given
    let mut value = ExtractStyleValue::Typography("heading".into());
    let expected = value.clone();
    // When
    value.join_naming(Naming::Risky);
    // Then
    assert_eq!(value, expected);
}

#[test]
fn assignment_binding_avoids_every_authored_prefix_collision() {
    // Given
    let _scope = crate::sparse_sites::SiteScope::enter(
        "collision.tsx",
        "let __devupAssignment17;let __devupAssignment17_;",
        &[],
    );
    // When
    let binding = crate::sparse_sites::binding_name(17);
    // Then: name must be safe as a JS binding, including the already suffixed name.
    assert_eq!(binding, "__devupAssignment17__");
}
