use super::literal_w38l_c1c_source::produced;
use crate::css_prop::{UNPLACED, template_parts};
use crate::utils::readable_code;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("", Ok(0))]
#[case(";", Ok(1))]
#[case(";;", Ok(1))]
#[case("${'margin:0'};", Ok(1))]
#[case("${'a'};", Ok(1))]
#[case("${css({color:'red'})};", Ok(1))]
#[case("color:${base};", Ok(1))]
#[case("${base};", Err(UNPLACED))]
#[case("&:hover{${base};}", Err(UNPLACED))]
#[serial]
fn default_entry_when_mixins_are_disabled_retains_whole_text_or_exact_rejection(
    #[case] body: &str,
    #[case] expected: Result<usize, &str>,
) {
    // Given: even the default control receives real parsed/visited producer input.
    let source = format!("import {{css}} from '@devup-ui/react';unknown`{body}`;");
    let allocator = Allocator::default();
    let (visitor, expression) = produced(&allocator, &source);
    let Expression::TaggedTemplateExpression(tag) = &expression else {
        panic!("retained source template")
    };
    let original = Expression::TemplateLiteral(oxc_allocator::Box::new_in(
        tag.quasi.clone_in_with_semantic_ids(&allocator),
        &visitor.ast,
    ));
    // When: a non-styled caller uses the actual default splitter entry.
    let actual = template_parts(&visitor.ast, &tag.quasi, false);
    // Then: text, punctuation and rejection placement are unchanged, not reinterpreted.
    match (actual, expected) {
        (Ok(parts), Ok(count)) => {
            assert_eq!(parts.len(), count);
            for part in parts {
                assert_eq!(part.span(), original.span());
                assert_eq!(readable_code(&part), readable_code(&original));
            }
        }
        (Err((hole, requirement)), Err(expected)) => {
            assert_eq!(requirement, expected);
            assert_eq!(hole.span(), tag.quasi.expressions[0].span());
            assert_eq!(
                readable_code(&hole),
                readable_code(&tag.quasi.expressions[0])
            );
        }
        other => panic!("unexpected default segmentation: {other:?}"),
    }
}
