use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("\r")]
#[case("\r\n")]
#[serial]
fn template_when_line_ending_normalizes_locates_raw_order(#[case] ending: &str) {
    // Given: real source bytes, not backslash-r text.
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nconst a=css`color:red;{ending}style-order:255`;"
    );
    // When: invalid metadata is reported.
    let actual = error(&source);
    // Then: original line and column survive Oxc's quasi normalization.
    assert!(actual.starts_with("a.tsx:3:13:"), "{actual}");
}

#[rstest]
#[case("\n")]
#[case("\r\n")]
#[case("\r")]
#[serial]
fn string_when_continuation_is_cooked_away_locates_raw_order(#[case] ending: &str) {
    // Given: a valid quoted JS string with a real line continuation.
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nconst a=css(\"color:red;\\{ending}style-order:255\");"
    );
    // When: strict metadata rejects 255.
    let actual = error(&source);
    // Then: the continuation is still counted in the original source location.
    assert!(actual.starts_with("a.tsx:3:13:"), "{actual}");
}

#[rstest]
#[case(r"\u{1F600}")]
#[case(r"\uD83D\uDE00")]
#[case(r"\u0041")]
#[case(r"\x41")]
#[serial]
fn string_when_escape_changes_cooked_length_locates_raw_order(#[case] escape: &str) {
    // Given: valid string escapes of distinct raw widths.
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nconst a=css(\"content:'{escape}';style-order:255\");"
    );
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find("255")
        .required("invalid order")
        + 1;
    // When: public extraction rejects the authored number.
    let actual = error(&source);
    // Then: location comes from raw bytes, not decoded text.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
fn generated_text_when_comment_cleaning_changes_quasi_uses_original_fallback() {
    use crate::css_utils::literal::CssText;
    use oxc_ast::ast::{Expression, ObjectPropertyKind, Statement};
    // Given: production cleanup changes an interior comment in a nested value.
    let source = "tag`style-order:2;color:red;_hover:style-/*gap*/order:${on?3:4}`;";
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression fixture")
    };
    let Expression::TaggedTemplateExpression(tag) = &statement.expression else {
        panic!("tag fixture")
    };
    let ast = oxc_ast::builder::AstBuilder::new(&allocator);
    let text = CssText::from_template(&ast, &tag.quasi, Some(source));
    let Expression::ObjectExpression(object) = text.scoped_object(&ast, 0..text.text.len(), false)
    else {
        panic!("produced object")
    };
    let value = object
        .properties
        .iter()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if property
                    .key
                    .static_name()
                    .is_some_and(|key| key == "_hover") =>
            {
                Some(&property.value)
            }
            _ => None,
        })
        .required("produced hover value");
    let Expression::TemplateLiteral(template) = value else {
        panic!("produced template")
    };
    // When: the actual generated template re-enters the source-aware constructor.
    let actual = CssText::with_source(&ast, value, Some(source)).required("re-entry");
    // Then: mismatched cleaned bytes use the supplied quasi's original start.
    assert_eq!(actual.offset(0), template.quasis[0].span.start);
    assert!(actual.text.starts_with("style-order:"));
    assert!(source.contains("style-/*gap*/order:"));
}
