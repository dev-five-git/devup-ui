use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::GetSpan;
use oxc_span::SourceType;

use crate::{
    extractor::extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    gen_class_name::gen_class_names,
    gen_style::gen_styles,
    provenance::SiteScope,
    utils::expression_to_code,
};

fn code(expression: &oxc_ast::ast::Expression<'_>) -> String {
    expression_to_code(expression)
        .trim()
        .trim_end_matches(';')
        .to_string()
}

pub(super) fn lowered(expression: &str) -> String {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = format!("({expression});");
    let _sites = SiteScope::enter("assignment.tsx", &source, &[]);
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut program = Parser::new(&allocator, &source, SourceType::ts())
        .parse()
        .program;
    let Statement::ExpressionStatement(statement) = &mut program.body[0] else {
        panic!("fixture expression");
    };
    let mut styles = extract_style_from_expression(
        &builder,
        Some("gap"),
        &mut statement.expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    )
    .styles;
    let class = gen_class_names(&builder, &mut styles, None, None)
        .map_or_else(|| "''".to_string(), |value| code(&value));
    let style =
        gen_styles(&builder, &styles, None).map_or_else(|| "{}".to_string(), |value| code(&value));
    let values = crate::assignment_lowering::take_evaluations(&mut styles);
    let params = values
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let arguments = values
        .iter()
        .map(|(_, value)| code(value))
        .collect::<Vec<_>>()
        .join(",");
    if values.is_empty() {
        format!("({{className:{class},style:{style}}})")
    } else {
        format!("(({params})=>({{className:{class},style:{style}}}))({arguments})")
    }
}

pub(super) fn evaluate(script: &str) -> String {
    Context::default()
        .eval(Source::from_bytes(script))
        .unwrap_or_else(|error| panic!("{error}\n{script}"))
        .as_string()
        .unwrap_or_else(|| panic!("fixture returns JSON"))
        .to_std_string_escaped()
}

struct JsxProps<'a>(AstBuilder<'a>);

impl<'a> oxc_ast_visit::VisitMut<'a> for JsxProps<'a> {
    fn visit_expression(&mut self, expression: &mut oxc_ast::ast::Expression<'a>) {
        use oxc_allocator::{CloneIn, GetAllocator};
        use oxc_ast::ast::{
            Expression, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, ObjectPropertyKind,
            PropertyKey, PropertyKind,
        };
        oxc_ast_visit::walk_mut::walk_expression(self, expression);
        let unwrapped = crate::utils::unwrap_syntax_only(expression);
        if unwrapped.span() != expression.span() {
            *expression = unwrapped.clone_in(self.0.allocator());
        }
        if let Expression::JSXElement(element) = expression {
            let builder = &self.0;
            let allocator = builder.allocator();
            let mut properties = oxc_allocator::Vec::new_in(builder);
            let tag = element.opening_element.name.to_string();
            if matches!(
                &element.opening_element.name,
                oxc_ast::ast::JSXElementName::MemberExpression(_)
            ) || tag.starts_with(|character: char| character.is_ascii_uppercase())
            {
                properties.push(ObjectPropertyKind::new_object_property(
                    oxc_span::SPAN,
                    PropertyKind::Init,
                    PropertyKey::new_static_identifier(oxc_span::SPAN, "tag", builder),
                    Expression::new_identifier(oxc_span::SPAN, allocator.alloc_str(&tag), builder),
                    false,
                    false,
                    false,
                    builder,
                ));
            }
            for attribute in &element.opening_element.attributes {
                match attribute {
                    JSXAttributeItem::Attribute(attribute) => {
                        let value = match &attribute.value {
                            Some(JSXAttributeValue::ExpressionContainer(container)) => container
                                .expression
                                .as_expression()
                                .map(|value| value.clone_in(allocator)),
                            Some(JSXAttributeValue::StringLiteral(value)) => {
                                Some(Expression::StringLiteral(value.clone_in(allocator)))
                            }
                            _ => None,
                        };
                        if let Some(value) = value {
                            let name = match &attribute.name {
                                JSXAttributeName::Identifier(name) => name.name.to_string(),
                                JSXAttributeName::NamespacedName(name) => {
                                    format!("{}:{}", name.namespace.name, name.name.name)
                                }
                            };
                            properties.push(ObjectPropertyKind::new_object_property(
                                oxc_span::SPAN,
                                PropertyKind::Init,
                                PropertyKey::new_string_literal(
                                    oxc_span::SPAN,
                                    allocator.alloc_str(&name),
                                    None,
                                    builder,
                                ),
                                value,
                                false,
                                false,
                                false,
                                builder,
                            ));
                        }
                    }
                    JSXAttributeItem::SpreadAttribute(spread) => {
                        properties.push(ObjectPropertyKind::new_spread_property(
                            oxc_span::SPAN,
                            spread.argument.clone_in(allocator),
                            builder,
                        ));
                    }
                }
            }
            let children = element
                .children
                .iter()
                .filter_map(|child| match child {
                    oxc_ast::ast::JSXChild::ExpressionContainer(container) => container
                        .expression
                        .as_expression()
                        .map(|value| value.clone_in(allocator)),
                    _ => None,
                })
                .map(oxc_ast::ast::ArrayExpressionElement::from);
            properties.push(ObjectPropertyKind::new_object_property(
                oxc_span::SPAN,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(oxc_span::SPAN, "children", builder),
                Expression::new_array_expression(
                    oxc_span::SPAN,
                    oxc_allocator::Vec::from_iter_in(children, builder),
                    builder,
                ),
                false,
                false,
                false,
                builder,
            ));
            *expression = Expression::new_object_expression(oxc_span::SPAN, properties, builder);
        }
    }
}

pub(super) fn extracted(source: &str) -> crate::ExtractOutput {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    crate::extract_without_source_map(
        "assignment.tsx",
        source,
        crate::ExtractOption {
            single_css: true,
            ..crate::ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

pub(super) fn compiled_jsx(source: &str) -> String {
    let output = extracted(source);
    jsx_js(&output.code)
}

pub(super) fn jsx_js(source: &str) -> String {
    use oxc_ast_visit::VisitMut;
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, source, SourceType::tsx())
        .parse()
        .program;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    JsxProps(AstBuilder::new(&allocator)).visit_program(&mut program);
    oxc_codegen::Codegen::new().build(&program).code
}
