use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::style_property::StyleProperty;
use crate::{ExtractOutput, ExtractStyleValue};
use oxc_ast::ast::{JSXAttribute, JSXAttributeValue};
use oxc_ast_visit::Visit;

#[derive(Default)]
struct Classes(String);

impl<'a> Visit<'a> for Classes {
    fn visit_jsx_attribute(&mut self, attribute: &JSXAttribute<'a>) {
        if attribute
            .name
            .as_identifier()
            .is_some_and(|name| name.name == "className")
            && let Some(JSXAttributeValue::ExpressionContainer(value)) = &attribute.value
            && let Some(expression) = value.expression.as_expression()
        {
            self.0 =
                crate::css_utils::rm_last_semi_colon(&crate::utils::expression_to_code(expression))
                    .to_string();
        }
    }
}

pub(crate) fn active(output: &ExtractOutput, props: &str) -> Vec<ExtractStyleValue> {
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, &output.code, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut classes = Classes::default();
    classes.visit_program(&parsed.program);
    assert_ne!(classes.0, "", "expected generated className expression");
    let names = evaluate(&classes.0, props);
    output
        .styles
        .iter()
        .filter(|value| {
            let class = match value {
                ExtractStyleValue::Static(style) => style.extract(None).to_string(),
                ExtractStyleValue::Dynamic(style) => {
                    let property = style.extract(None);
                    assert!(
                        matches!(property, StyleProperty::Variable { .. }),
                        "dynamic atom requires a variable"
                    );
                    match property {
                        StyleProperty::Variable { class_name, .. }
                        | StyleProperty::ClassName(class_name) => class_name,
                    }
                }
                other => panic!("expected static atom, got {other:?}"),
            };
            names.split_whitespace().any(|name| name == class)
        })
        .cloned()
        .collect()
}

pub(crate) fn evaluate(expression: &str, props: &str) -> String {
    let code = format!("const rest = {props}; const className = ''; ({expression});");
    let mut context = boa_engine::Context::default();
    context
        .eval(boa_engine::Source::from_bytes(&code))
        .unwrap_or_else(|error| panic!("{error}: {code}"))
        .to_string(&mut context)
        .unwrap_or_else(|error| panic!("{error}"))
        .to_std_string_escaped()
}

pub(crate) fn declarations(styles: &[ExtractStyleValue]) -> Vec<(String, String)> {
    let mut declarations: Vec<_> = styles
        .iter()
        .map(|value| match value {
            ExtractStyleValue::Static(style) => {
                (style.property().to_string(), style.value().to_string())
            }
            other => panic!("expected static atom, got {other:?}"),
        })
        .collect();
    declarations.sort();
    declarations
}

#[rstest::rstest]
#[case("<div />")]
#[case("<div className />")]
#[case("<div className='a' />")]
#[case("<div className={/* missing expression */} />")]
fn active_oracle_when_generated_class_expression_is_missing_rejects(
    #[case] jsx: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut output = crate::extract(
        "oracle.tsx",
        "const value = 1;",
        crate::ExtractOption::default(),
    )?;
    output.code = format!("const Choice = () => {jsx};");
    // When
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| active(&output, "{}")));
    // Then
    assert!(
        result.is_err(),
        "missing classes must not become empty styles"
    );
    Ok(())
}
