use css::{
    file_map::{get_or_insert_original_id, reset_file_map},
    style_selector::StyleSelector,
};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_ast_visit::VisitMut;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::{
    ProducerPolicy,
    extract_style_value::ExtractStyleValue,
    policy_test_support::{expression, set_lengths},
};
use crate::{
    ExtractStyleProp,
    extractor::{
        extract_keyframes_from_expression::extract_keyframes_from_expression,
        extract_style_from_expression::{
            LiteralHandling, dynamic_style, extract_style_from_expression,
        },
        extract_style_from_stylex::extract_stylex_namespace_styles,
    },
    sparse_sites::SiteScope,
    visit::DevupVisitor,
};

#[test]
#[serial]
fn conditional_typography_literals_retain_originals_when_real_branches_are_extracted() {
    // Given: registered originals and actual preset branches, not manually built typography atoms.
    reset_file_map();
    let ids = ["policy-typo-a.tsx", "policy-typo-b.tsx"]
        .map(|name| get_or_insert_original_id(name).unwrap_or_else(|_| panic!("register fixture")));
    let previous = css::theme_tokens::get_typography_keys();
    css::theme_tokens::set_typography_keys(vec!["body".into(), "title".into()]);
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let selector = Some(StyleSelector::from("&:hover"));
    let mut records = vec![];
    // When: the real literal and dynamic typography paths construct their conditional atoms.
    for original in ids {
        let _scope = SiteScope::enter_counter_numbered(original, "tone", &[]);
        let mut literal = expression(&allocator, "('body');");
        records.extend(
            extract_style_from_expression(
                &builder,
                Some("typography"),
                &mut literal,
                2,
                &selector,
                LiteralHandling::ExpandResponsiveThemeToken,
            )
            .styles
            .into_iter()
            .flat_map(ExtractStyleProp::into_extract),
        );
        let condition = expression(&allocator, "tone;");
        records
            .extend(dynamic_style(&builder, "typography", &condition, 2, &selector).into_extract());
    }
    // Then: each owner retains both presets under the original selector/level; duplicates dedup only within an owner.
    assert_eq!(records.len(), 6);
    assert_eq!(set_lengths(&records), (4, 4));
    for (original, records) in ids.into_iter().zip(records.as_chunks::<3>().0) {
        for record in records {
            let ExtractStyleValue::Static(style) = record else {
                panic!("conditional typography atom");
            };
            assert_eq!(
                style.producer_policy(),
                ProducerPolicy::CounterOriginal(original)
            );
            assert_eq!((style.property(), style.level()), ("typography", 2));
            assert_eq!(style.selector(), selector.as_ref());
            assert!(matches!(style.value(), "body" | "title"));
        }
    }
    css::theme_tokens::set_typography_keys(previous);
    reset_file_map();
}

#[test]
#[serial]
fn raw_stylex_literals_retain_originals_when_numeric_css_bypasses_devup_scaling() {
    // Given: raw StyleX numbers would be scaled incorrectly by replacing its literal with new().
    reset_file_map();
    let ids = ["policy-stylex-a.tsx", "policy-stylex-b.tsx"]
        .map(|name| get_or_insert_original_id(name).unwrap_or_else(|_| panic!("register fixture")));
    let allocator = Allocator::default();
    let source = "({card:{paddingLeft:4}});";
    let parsed = expression(&allocator, source);
    let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(&parsed) else {
        panic!("StyleX object");
    };
    let mut records = vec![];
    let mut errors = vec![];
    // When: real StyleX namespaces are extracted before the union.
    for original in ids {
        let _scope = SiteScope::enter_counter_numbered(original, source, &[]);
        for (_, styles, _, _) in extract_stylex_namespace_styles(
            object,
            &Default::default(),
            &Default::default(),
            &mut errors,
        ) {
            records.extend(styles.into_iter().flat_map(ExtractStyleProp::into_extract));
        }
    }
    // Then: both raw 4px declarations retain distinct originals, rather than scaled 16px.
    assert_eq!(errors, vec![]);
    assert_eq!(set_lengths(&records), (2, 2));
    assert_eq!(records.len(), 2);
    for (record, original) in records.iter().zip(ids) {
        let ExtractStyleValue::Static(style) = record else {
            panic!("raw StyleX atom");
        };
        assert_eq!((style.property(), style.value()), ("padding-left", "4px"));
        assert_eq!(
            style.producer_policy(),
            ProducerPolicy::CounterOriginal(original)
        );
    }
    reset_file_map();
}

#[rstest]
#[case("({});", 0)]
#[case("tone;", 0)]
#[case("({to:{opacity:1},from:{opacity:0}});", 2)]
fn object_keyframes_capture_parent_and_children_when_real_expression_is_read(
    #[case] source: &str,
    #[case] steps: usize,
) {
    // Given: empty, unreadable and populated object-expression keyframe inputs.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut input = expression(&allocator, source);
    let _scope = SiteScope::enter_counter_numbered(7, source, &[]);
    // When: the actual extractor constructs its default then fills ordered steps.
    let frames = extract_keyframes_from_expression(&builder, &mut input).keyframes;
    // Then: the parent retains an original even without steps; children retain the same capture.
    assert_eq!(frames.producer_policy(), ProducerPolicy::CounterOriginal(7));
    assert_eq!(frames.keyframes.len(), steps);
    for styles in frames.keyframes.values() {
        assert_eq!(styles.len(), 1);
        assert_eq!(
            styles[0].producer_policy(),
            ProducerPolicy::CounterOriginal(7)
        );
        assert_eq!(styles[0].property(), "opacity");
    }
    if steps == 2 {
        assert_eq!(
            frames
                .keyframes
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["from", "to"]
        );
    }
}

#[rstest]
#[case("", 0)]
#[case("from { opacity: 0; } to { opacity: 1; }", 2)]
#[serial]
fn template_keyframe_literal_captures_original_when_visitor_inserts_record(
    #[case] css: &str,
    #[case] steps: usize,
) {
    // Given: registered original authority and the actual tagged-template visitor path.
    reset_file_map();
    css::class_map::reset_class_map();
    let original = get_or_insert_original_id("policy-frames.tsx")
        .unwrap_or_else(|_| panic!("register fixture"));
    let source =
        format!("import {{keyframes}} from '@devup-ui/react'; const frames=keyframes`{css}`;");
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let _scope = SiteScope::enter_counter_numbered(original, &source, &[]);
    let mut visitor = DevupVisitor::new(
        &allocator,
        "policy-frames.tsx",
        "@devup-ui/react",
        vec![],
        None,
    );
    // When: the visitor constructs and inserts template frames through the real literal seam.
    visitor.visit_program(&mut parsed.program);
    // Then: even an empty template retains the captured parent original before set insertion.
    assert_eq!(visitor.errors, vec![]);
    assert_eq!(visitor.styles.len(), 1);
    let Some(ExtractStyleValue::Keyframes(frames)) = visitor.styles.iter().next() else {
        panic!("template frames");
    };
    assert_eq!(
        frames.producer_policy(),
        ProducerPolicy::CounterOriginal(original)
    );
    assert_eq!(frames.keyframes.len(), steps);
    for styles in frames.keyframes.values() {
        for style in styles {
            assert_eq!(
                style.producer_policy(),
                ProducerPolicy::CounterOriginal(original)
            );
        }
    }
    reset_file_map();
    css::class_map::reset_class_map();
}
