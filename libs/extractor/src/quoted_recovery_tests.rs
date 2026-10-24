use crate::{
    css_prop::template_parts,
    css_utils::{
        CssToStyleResult, Place, css_to_style, css_to_style_literal, interpolation_place,
        optimize_css_block,
    },
    extract_style::extract_static_style::ExtractStaticStyle,
    utils::expression_to_code,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("'open  ")]
#[case("\"한글 0.5 \t")]
#[case("'a\\\r\n  ")]
fn quoted_recovery_keeps_eof_payload_when_parsing_declarations(#[case] value: &str) {
    // Given CSS Syntax EOF string content: https://drafts.csswg.org/css-syntax-3/#consume-string-token
    let source = format!("content: {value}");
    // When a declaration is parsed into static styles.
    let styles = css_to_style(&source, 0, &None);
    // Then trimming at every parser boundary preserves the trailing payload.
    assert_eq!(
        styles
            .iter()
            .map(ExtractStaticStyle::value)
            .collect::<Vec<_>>(),
        vec![value]
    );
}

#[test]
fn quoted_recovery_keeps_eof_payload_when_optimizing_blocks() {
    // Given an EOF-open string at the end of a raw CSS block.
    let source = "content: 'open  ";
    // When declaration formatting is optimized.
    let actual = optimize_css_block(source);
    // Then the final whole-block trim preserves the string's spaces.
    assert_eq!(actual, "content:'open  ");
}

#[test]
fn quoted_recovery_keeps_quoted_colons_when_an_optimized_block_ends_at_eof() {
    // Given an incomplete selector with a colon inside its EOF string.
    let source = "&[title='a:b  ";
    // When raw block optimization sees no actual declaration or rule body.
    let actual = optimize_css_block(source);
    // Then the quoted colon cannot split off payload for generic value trimming.
    assert_eq!(actual, source);
}

#[rstest]
fn quoted_recovery_parses_following_syntax_when_a_string_hits_a_newline(
    #[values("\n", "\r", "\x0c", "\r\n")] newline: &str,
) {
    // Given a bad string before a real separator, comment and declaration.
    let source = format!("content:'bad{newline};/* gone */opacity:0.5");
    // When the CSS block is parsed.
    let styles = css_to_style(&source, 0, &None);
    // Then following syntax is recovered without deleting the raw newline.
    let actual: Vec<_> = styles
        .iter()
        .map(|style| (style.property().to_string(), style.value().to_string()))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("content".to_string(), format!("'bad{newline}")),
            ("opacity".to_string(), ".5".to_string())
        ]
    );
}

#[rstest]
fn quoted_recovery_places_mixins_when_a_newline_crosses_quasis(
    #[values("\n", "\r", "\x0c")] newline: &str,
) {
    // Given a string opened in one quasi and terminated in the following quasi.
    let source = format!("`content:'${{label}}{newline}; ${{base}};`");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(expression) = &parsed.program.body[0] else {
        panic!("expected expression");
    };
    let Expression::TemplateLiteral(template) = &expression.expression else {
        panic!("expected template");
    };
    let before = format!(
        "{}{}",
        template.quasis[0].value.raw, template.quasis[1].value.raw
    );
    // When placement scans the boundary across those quasis.
    let actual = interpolation_place(&before, &template.quasis[2..]);
    // Then the following interpolation is a statement, not string value data.
    assert!(matches!(actual, Place::Statement));
}

#[rstest]
fn quoted_recovery_splits_mixins_when_a_bad_string_ends_in_a_later_quasi(
    #[values("\n", "\r", "\x0c")] newline: &str,
) {
    // Given the same malformed string in a css-prop template.
    let source = format!("`content:'${{label}}{newline}; ${{base}};`");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(expression) = &parsed.program.body[0] else {
        panic!("expected expression");
    };
    let Expression::TemplateLiteral(template) = &expression.expression else {
        panic!("expected template");
    };
    // When the css prop is decomposed around top-level mixins.
    let Ok(actual) = template_parts(&AstBuilder::new(&allocator), template, true) else {
        panic!("expected top-level mixin parts");
    };
    // Then the recovered mixin is represented as its own expression.
    assert_eq!(
        actual.get(1).map(expression_to_code),
        Some("base;".to_string())
    );
}

#[test]
fn quoted_recovery_selects_dependencies_when_a_bad_string_ended_in_an_earlier_quasi() {
    // Given a selector interpolation after a string's terminating newline.
    let source = concat!(
        "css`content:'${",
        "value}\n; ${",
        "selector}{",
        "color:red}`"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let mut selected = Vec::new();
    // When dependencies are classified before extraction.
    crate::style_values::selectors(&parsed.program, &mut |expression| {
        selected.push(crate::utils::readable_code(expression));
    });
    // Then only the recovered selector dependency is selected.
    assert_eq!(selected, vec!["selector"]);
}

#[test]
fn quoted_recovery_keeps_eof_payload_when_extracting_a_template() {
    // Given an EOF-open CSS string ending at the JavaScript template boundary.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "`content:'open  `", SourceType::ts()).parse();
    let Statement::ExpressionStatement(expression) = &parsed.program.body[0] else {
        panic!("expected expression");
    };
    let Expression::TemplateLiteral(template) = &expression.expression else {
        panic!("expected template");
    };
    // When the template is extracted into static declarations.
    let styles = css_to_style_literal(template, 0, &None);
    // Then the original trailing CSS string bytes reach the extracted value.
    let [CssToStyleResult::Static(style)] = styles.as_slice() else {
        panic!("expected one static style");
    };
    assert_eq!(style.value(), "'open  ");
}
