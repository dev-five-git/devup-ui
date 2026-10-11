use crate::extract_style::compiler_associations as associations;
use crate::extract_style::compiler_projection;
use crate::extractor::extract_style_from_expression::yield_typography;
use crate::{ExtractStyleProp, ExtractStyleValue as Value};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{Expression, Str, TemplateElement, TemplateElementValue};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, GetSpanMut, SPAN};

#[path = "gen_class_name_branches.rs"]
mod branches;

#[derive(Clone, Debug)]
pub struct OrderEdge {
    pub before: Value,
    pub after: Value,
    pub receipt: crate::extract_style::compiler_receipts::ReceiptId,
    pub invocation: crate::extract_style::compiler_associations::InvocationId,
    pub scope: Option<crate::extract_style::compiler_associations::ScopeId>,
}
pub(crate) fn class_name(value: &Value, filename: Option<&str>) -> Option<String> {
    projected_class(value, filename, None)
}
fn projected_class(
    value: &Value,
    filename: Option<&str>,
    before: Option<&Value>,
) -> Option<String> {
    use crate::extract_style::compiler_receipts as receipts;
    use crate::extract_style::style_property::StyleProperty;
    if !crate::compiler_policy::active() {
        return value.extract(filename).map(|value| match value {
            StyleProperty::ClassName(name)
            | StyleProperty::Variable {
                class_name: name, ..
            } => name,
        });
    }
    match value {
        Value::Typography(name) => Some(format!("typo-{name}")),
        Value::Css(_) | Value::Import(_) | Value::FontFace(_) => None,
        Value::Static(_) | Value::Dynamic(_) | Value::Keyframes(_) => {
            let result = compiler_projection::context(value, filename)
                .and_then(|context| receipts::acquire(value, filename, context));
            receipts::collect_operand(result, value).map(|(receipt, produced)| {
                associations::ordered(before.unwrap_or(value), value, receipt);
                produced.class().allocation.name.clone()
            })
        }
    }
}

pub(crate) fn keyframe_name(
    frames: &crate::extract_style::ExtractKeyframes,
    filename: Option<&str>,
) -> Option<String> {
    use crate::extract_style::ExtractStyleProperty;
    if !crate::compiler_policy::active() {
        return Some(crate::visit::style_property_into_string(
            frames.extract(filename),
        ));
    }
    class_name(&Value::Keyframes(frames.clone()), filename)
}
pub(crate) fn static_name(
    style: &crate::extract_style::extract_static_style::ExtractStaticStyle,
    filename: Option<&str>,
) -> Option<String> {
    class_name(&Value::Static(style.clone()), filename)
}

pub fn gen_class_names<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    yield_typography(style_props);
    crate::assignment_lowering::coalesce(style_props);
    merge_expression_for_class_name(
        ast_builder,
        style_props
            .iter_mut()
            .filter_map(|st| gen_class_name(ast_builder, st, style_order, filename))
            .rev(),
    )
}

fn gen_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &mut ExtractStyleProp<'a>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    let projection = branches::ClassProjection {
        ast: ast_builder,
        order: style_order,
        filename,
    };
    match style_prop {
        ExtractStyleProp::Evaluated {
            source,
            styles,
            binding,
            evaluation,
            alternate_order,
            alternate_class,
        } => {
            crate::assignment_consumers::merge(styles);
            let mut value = crate::assignment_lowering::Lowering {
                ast: ast_builder,
                order: style_order,
                filename,
                alternate_order: *alternate_order,
            }
            .lower(source, styles);
            let _ = gen_class_names(ast_builder, styles, style_order, filename);
            *value.span_mut() = source.span();
            *evaluation = Some(value);
            Some(crate::assignment_lowering::projection(
                ast_builder,
                binding,
                if *alternate_class { 3 } else { 0 },
            ))
        }
        ExtractStyleProp::Enum { map, condition } => Some(projection.enum_class(map, condition)),
        ExtractStyleProp::Static(st) => {
            let before = crate::compiler_policy::active().then(|| st.clone());
            if let Some(style_order) = style_order {
                st.set_style_order(style_order);
            }
            projected_class(st, filename, before.as_ref()).map(|name| {
                let v = Str::from_in(&name, ast_builder.allocator());
                Expression::new_string_literal(SPAN, v, None, ast_builder)
            })
        }
        ExtractStyleProp::StaticArray(res) => merge_expression_for_class_name(
            ast_builder,
            res.iter_mut()
                .filter_map(|st| gen_class_name(ast_builder, st, style_order, filename)),
        ),
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
            ..
        } => Some(projection.conditional_class(condition, (consequent, alternate))),
        ExtractStyleProp::Expression { expression, .. } => {
            Some(expression.clone_in(ast_builder.allocator()))
        }
        ExtractStyleProp::Unreadable { .. } => None,
        // direct select
        ExtractStyleProp::MemberExpression { map, expression } => {
            Some(projection.member_class(map, expression))
        }
    }
}

pub fn merge_expression_for_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: impl IntoIterator<Item = Expression<'a>>,
) -> Option<Expression<'a>> {
    let mut unknown_expr: Vec<Expression<'a>> = Vec::new();
    let mut class_name = String::new();
    for expr in expressions {
        if let Expression::StringLiteral(str) = &expr {
            let value = str.value.trim();
            if !value.is_empty() {
                if class_name.is_empty() {
                    // First fragment: reserve up front so appending the initial static
                    // classes does not trigger an early grow-realloc. Subsequent
                    // fragments rely on amortized growth.
                    class_name.reserve(value.len() + 4);
                } else {
                    class_name.push(' ');
                }
                class_name.push_str(value);
            }
        } else {
            unknown_expr.push(expr);
        }
    }
    if unknown_expr.is_empty() {
        if class_name.is_empty() {
            return None;
        }
        return Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(&class_name, ast_builder.allocator()),
            None,
            ast_builder,
        ));
    }
    if class_name.is_empty() && unknown_expr.len() == 1 {
        Some(unknown_expr.remove(0))
    } else {
        // Decide the head quasi once, up front. When there are static classes we
        // append a trailing space so the first interpolation is separated; when
        // there are none the head is empty. Hoisting this out of the loop removes
        // the per-`idx == 0` `class_name.is_empty()` re-test. Output is byte-identical.
        let head: &str = if class_name.is_empty() {
            ""
        } else {
            class_name.push(' ');
            class_name.as_str()
        };
        let mut qu = oxc_allocator::Vec::new_in(ast_builder);
        for idx in 0..=unknown_expr.len() {
            let tail = idx == unknown_expr.len();
            let t = TemplateElementValue {
                raw: Str::from_in(
                    if idx == 0 {
                        head
                    } else if tail {
                        ""
                    } else {
                        " "
                    },
                    ast_builder.allocator(),
                ),
                cooked: None,
            };
            qu.push(TemplateElement::new(SPAN, t, tail, ast_builder));
        }

        Some(Expression::new_template_literal(
            SPAN,
            qu,
            oxc_allocator::Vec::from_iter_in(unknown_expr, ast_builder),
            ast_builder,
        ))
    }
}
