use oxc_allocator::Allocator;
use rstest::rstest;

use super::{
    ProducerPolicy, extract_dynamic_style::ExtractDynamicStyle,
    extract_keyframes::ExtractKeyframes, extract_static_style::ExtractStaticStyle,
    extract_style_value::ExtractStyleValue, policy_test_support::expression,
};
use crate::{ExtractStyleProp, sparse_sites::SiteScope};

#[derive(Clone, Copy, Debug)]
enum Wrapper {
    Array,
    Evaluated,
    Expression,
    Member,
    Enum,
    Conditional,
}

fn wrap(
    kind: Wrapper,
    styles: Vec<ExtractStyleValue>,
    allocator: &Allocator,
) -> ExtractStyleProp<'_> {
    let source = expression(allocator, "tone;");
    let props = || {
        styles
            .iter()
            .cloned()
            .map(ExtractStyleProp::Static)
            .collect::<Vec<_>>()
    };
    match kind {
        Wrapper::Array => ExtractStyleProp::StaticArray(props()),
        Wrapper::Evaluated => ExtractStyleProp::Evaluated {
            styles: props(),
            source,
            binding: "captured".into(),
            evaluation: None,
            alternate_order: None,
            alternate_class: false,
        },
        Wrapper::Expression => ExtractStyleProp::Expression {
            styles,
            expression: source,
        },
        Wrapper::Member => ExtractStyleProp::MemberExpression {
            map: [(
                "selected".into(),
                Box::new(ExtractStyleProp::StaticArray(props())),
            )]
            .into(),
            expression: source,
        },
        Wrapper::Enum => ExtractStyleProp::Enum {
            condition: source,
            map: [("selected".into(), props())].into(),
        },
        Wrapper::Conditional => ExtractStyleProp::Conditional {
            condition: source,
            consequent: Some(Box::new(ExtractStyleProp::StaticArray(props()))),
            alternate: None,
        },
    }
}

#[rstest]
#[case(Wrapper::Array)]
#[case(Wrapper::Evaluated)]
#[case(Wrapper::Expression)]
#[case(Wrapper::Member)]
#[case(Wrapper::Enum)]
#[case(Wrapper::Conditional)]
fn deferred_records_retain_policy_when_wrapper_is_cloned_and_extracted(#[case] kind: Wrapper) {
    // Given: mixed originals, including a no-site dynamic and empty keyframes.
    let allocator = Allocator::default();
    let records = {
        let _outer = SiteScope::enter_counter_numbered(7, "outer", &[]);
        let style =
            ExtractStyleValue::Static(ExtractStaticStyle::new_basic("color", "red", 0, None));
        let dynamic = {
            let _inner = SiteScope::enter_counter_numbered(9, "inner", &[]);
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "tone", None))
        };
        vec![
            style,
            dynamic,
            ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
        ]
    };
    let wrapped = wrap(kind, records, &allocator);
    let _current = SiteScope::enter_numbered(42, "current", &[]);
    // When: AST cloning plus borrowed/owning extraction occurs under a different Current context.
    let cloned = wrapped.clone_in(&allocator);
    let borrowed = cloned.extract();
    let owned = cloned.into_extract();
    // Then: recursive projections preserve original captures and diagnostic/site absence.
    assert_eq!(borrowed, owned);
    assert_eq!(borrowed.len(), 3);
    let expected = [7, 9, 7];
    for (record, original) in borrowed.iter().zip(expected) {
        let policy = match record {
            ExtractStyleValue::Static(style) => style.producer_policy(),
            ExtractStyleValue::Dynamic(style) => {
                assert_eq!(style.site(), None);
                style.producer_policy()
            }
            ExtractStyleValue::Keyframes(frames) => {
                assert_eq!(frames.keyframes.len(), 0);
                frames.producer_policy()
            }
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_) => panic!("unexpected deferred record"),
        };
        assert_eq!(policy, ProducerPolicy::CounterOriginal(original));
    }
}
