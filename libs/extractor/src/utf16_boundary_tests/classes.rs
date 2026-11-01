use super::*;
use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};
use oxc_ast::ast::{JSXAttribute, JSXAttributeValue, VariableDeclarator};
use oxc_ast_visit::{Visit, walk};

fn rendered_class(code: &str) -> Vec<u16> {
    struct ClassExpression(Option<String>);
    impl<'a> Visit<'a> for ClassExpression {
        fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
            if declarator.id.get_identifier_name() == Some("result".into())
                && let Some(
                    expression @ (Expression::StringLiteral(_) | Expression::TemplateLiteral(_)),
                ) = &declarator.init
            {
                self.0 = Some(crate::utils::expression_to_code(expression));
            }
            walk::walk_variable_declarator(self, declarator);
        }
        fn visit_jsx_attribute(&mut self, attribute: &JSXAttribute<'a>) {
            if attribute
                .name
                .as_identifier()
                .is_some_and(|name| name.name == "className")
                && let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value
                && let Some(expression) = container.expression.as_expression()
            {
                self.0 = Some(crate::utils::expression_to_code(expression));
            }
        }
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{code}");
    let mut class = ClassExpression(None);
    class.visit_program(&parsed.program);
    let expression = class
        .0
        .unwrap_or_else(|| panic!("missing class expression: {code}"));
    let script = format!("let name = 'active'; {expression}");
    let value = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{script}: {error}"));
    value
        .as_string()
        .unwrap_or_else(|| panic!("expected class string: {script}"))
        .iter()
        .collect()
}

#[rstest]
#[case(r"'\uD800'", &[0xD800])]
#[case(r"`\uD800 ${name} \uDC00`", &[0xD800, 32, 97, 99, 116, 105, 118, 101, 32, 0xDC00])]
#[serial]
fn class_arguments_preserve_utf16_when_not_readable_as_utf8(
    #[case] argument: &str,
    #[case] expected: &[u16],
) {
    // Given a class-only Emotion call, with no CSS interpretation of its strings.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ cx }} from '@devup-ui/react/compat'; export const result = cx({argument});"
    );
    // When the class arguments are lowered.
    let output = extract("classes.tsx", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then executing the generated class expression preserves every UTF-16 code unit.
    assert_eq!(rendered_class(&output.code), expected);
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case(r"'\uD800'", &[0xD800])]
#[case(r"`\uD800 ${name} \uDC00`", &[0xD800, 32, 97, 99, 116, 105, 118, 101, 32, 0xDC00])]
#[serial]
fn tailwind_class_merge_keeps_surrogates_separate_from_generated_classes(
    #[case] class: &str,
    #[case] expected: &[u16],
) {
    // Given an authored opaque class and a generated padding class.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ Box }} from '@devup-ui/react'; export const result = <Box className={{{class}}} p={{2}} />;"
    );
    // When the element's generated and authored class lists are merged.
    let output = extract("classes.tsx", &source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("{error}"));
    // Then exact code units precede a space and the actual generated padding class.
    let padding = output
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "padding" => Some(style),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing padding"));
    assert_eq!(padding.value(), "8px");
    let mut expected = expected.to_vec();
    expected.push(32);
    let StyleProperty::ClassName(class) = padding.extract(Some("classes.tsx")) else {
        panic!("expected static class")
    };
    expected.extend(class.encode_utf16());
    assert_eq!(rendered_class(&output.code), expected);
}
