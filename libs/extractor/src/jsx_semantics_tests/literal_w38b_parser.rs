use super::*;
use crate::css_utils::{self, literal::CssText};
use oxc_allocator::Allocator;
use oxc_ast::{
    ast::{Expression, Statement, TemplateLiteral},
    builder::AstBuilder,
};
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

fn parsed_tag<'a>(allocator: &'a Allocator, source: &'a str) -> TemplateLiteral<'a> {
    let parsed = oxc_parser::Parser::new(allocator, source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    match parsed
        .program
        .body
        .into_iter()
        .next()
        .required("expression statement")
    {
        Statement::ExpressionStatement(statement) => match statement.unbox().expression {
            Expression::TaggedTemplateExpression(tag) => tag.unbox().quasi,
            _ => panic!("tagged expression required"),
        },
        _ => panic!("expression statement required"),
    }
}

#[rstest]
#[case("tag`${function(){return 'external'}};color:blue`; ")]
#[case("tag`${class External{}};color:blue`; ")]
fn statement_hole_when_parser_supplies_function_or_class_preserves_blue(#[case] source: &str) {
    // Given: actual template statement holes, not supported arbitrary callbacks.
    let allocator = Allocator::default();
    let template = parsed_tag(&allocator, source);
    // When: the production template placement operation reads the parser's quasi.
    let actual = css_utils::css_to_style_template(&template, 0, &None);
    // Then: statement index and static sibling declaration are preserved.
    assert_eq!(actual.statements, vec![0]);
    assert_eq!(actual.unplaced, Vec::<usize>::new());
    assert!(actual.styles.iter().any(|style| matches!(style, css_utils::CssToStyleResult::Static(style) if style.property()=="color" && style.value()=="blue")));
}

#[test]
fn declaration_when_raw_tag_has_metadata_preserves_custom_property_data() {
    // Given: a real raw CSS escape and token data containing a directive spelling.
    let allocator = Allocator::default();
    let template = parsed_tag(
        &allocator,
        r"tag`\73 tyle-order:2;co/*gap*/lor:red;--data:{style-order:9;}`;",
    );
    // When: the declaration producer processes its actual key/value ranges.
    let actual = css_utils::css_to_style(&template.quasis[0].value.raw, 0, &None);
    // Then: only metadata is removed; comment cleanup and opaque token data survive.
    let values: Vec<_> = actual
        .iter()
        .map(|style| (style.property(), style.value()))
        .collect();
    assert_eq!(
        values,
        vec![("--data", "{style-order:9;}"), ("color", "red")]
    );
}

#[test]
fn opaque_text_when_metadata_is_escaped_preserves_quoted_content() {
    // Given: metadata and quoted content share a spelling but not a syntactic role.
    let allocator = Allocator::default();
    let template = parsed_tag(
        &allocator,
        r"tag`@opaque{\73 tyle-order:2;content:'style-order:9';color:red}`;",
    );
    // When: the real opaque optimizer processes raw text.
    let actual = css_utils::optimize_css_block(&template.quasis[0].value.raw);
    // Then: metadata is absent while content remains data.
    assert!(
        actual.contains("content:style-order:9") && actual.contains("color:red"),
        "{actual}"
    );
    assert!(!actual.contains("style-order:2"), "{actual}");
}

#[rstest]
#[case("tag`style-order:2;co${key}:red`;", "key")]
#[case(concat!("tag`style-order:2;", "${selector}{color:red}`;"), "selector")]
fn scoped_text_when_public_preflight_bypasses_it_retains_original_hole(
    #[case] source: &str,
    #[case] token: &str,
) {
    // Given: public preflight may reject before scoped-object generation.
    let allocator = Allocator::default();
    let template = parsed_tag(&allocator, source);
    let ast = AstBuilder::new(&allocator);
    let text = CssText::from_template(&ast, &template, Some(source));
    // When: the supplying producer generates the real unplaced-key envelope.
    let actual = text.scoped_object(&ast, 0..text.text.len(), false);
    // Then: no manufactured AST/span is required to preserve the authored hole.
    let Expression::ObjectExpression(object) = actual else {
        panic!("produced object")
    };
    let sentinel = object
        .properties
        .iter()
        .find_map(|property| match property {
            oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property)
                if property
                    .key
                    .static_name()
                    .is_some_and(|key| key == "__devupLiteralUnplaced") =>
            {
                Some(&property.value)
            }
            _ => None,
        })
        .required("unplaced hole producer");
    assert_eq!(crate::utils::readable_code(sentinel), token);
    assert_eq!(sentinel.span().start, template.expressions[0].span().start);
}

#[rstest]
#[case("tag`selectors:red;body{style-order:2;color:blue}`;", None)]
#[case("tag`selectors:${{...rest}};body{style-order:2;color:blue}`;", None)]
#[case(
    "tag`selectors:${{[key]:'red'}};body{style-order:2;color:blue}`;",
    None
)]
#[case(
    "tag`@page{size:${{...rest}}}body{style-order:2;color:blue}`;",
    Some("@page{size{}}")
)]
#[serial]
fn global_text_when_exact_hole_clones_objects_preserves_sibling_and_opaque_contract(
    #[case] source: &str,
    #[case] opaque: Option<&str>,
) {
    // Given: the real producer can clone arbitrary objects into static containers.
    let allocator = Allocator::default();
    let template = parsed_tag(&allocator, source);
    let ast = AstBuilder::new(&allocator);
    let text = CssText::from_template(&ast, &template, Some(source));
    // When: parser-fed global extraction observes the actual cloned values.
    let actual = css_utils::global::extract_text(&ast, &text, "a.tsx").required("ordered text");
    // Then: the static sibling survives; opaque spread serialization remains unchanged.
    let styles: Vec<_> = actual.iter().flat_map(ExtractStyleProp::extract).collect();
    assert!(styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value()=="blue" && style.style_order()==Some(2))));
    let raw: Vec<_> = styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Css(css) => Some(css.css.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(raw, opaque.into_iter().collect::<Vec<_>>());
    if source.contains("[key]") {
        assert!(actual.iter().any(|prop| matches!(prop, ExtractStyleProp::Unreadable { offset, code, .. } if usize::try_from(*offset).required("offset fits")==source.find("key").required("authored key") && code=="[key]")), "{actual:?}");
    }
}

#[test]
#[serial]
fn selector_record_when_exact_hole_has_computed_key_rejects_authored_key() {
    // Given: a literal selectors declaration clones a real computed-key object.
    let source = "tag`selectors:${{[key]:{color:'red'}}};body{style-order:2;color:blue}`;";
    let allocator = Allocator::default();
    let template = parsed_tag(&allocator, source);
    let ast = AstBuilder::new(&allocator);
    let text = CssText::from_template(&ast, &template, Some(source));
    // When: the unguarded literal-record caller receives this actual producer output.
    let actual = css_utils::global::extract_text(&ast, &text, "a.tsx").required("ordered text");
    // Then: unreadability points to the original computed key.
    assert!(actual.iter().any(|prop| matches!(prop, ExtractStyleProp::Unreadable { offset, code, .. } if usize::try_from(*offset).required("offset fits")==source.find("key").required("authored key") && code=="[key]")), "{actual:?}");
}
