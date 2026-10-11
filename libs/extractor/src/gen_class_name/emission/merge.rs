use super::super::roots::ClassMergeRoot;
use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{Str, StringLiteral, TemplateElement, TemplateElementValue, TemplateLiteral};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;

pub(crate) fn merge_roots<'a, R: ClassMergeRoot<'a>>(
    ast: &AstBuilder<'a>,
    roots: impl IntoIterator<Item = R>,
) -> Option<R> {
    let mut dynamic: Vec<R> = Vec::new();
    let mut class_name = String::new();
    for root in roots {
        match root.merge_string_value() {
            Some(value) => {
                let value = value.trim();
                if !value.is_empty() {
                    if class_name.is_empty() {
                        class_name.reserve(value.len() + 4);
                    } else {
                        class_name.push(' ');
                    }
                    class_name.push_str(value);
                }
            }
            None => dynamic.push(root),
        }
    }
    if dynamic.is_empty() {
        if class_name.is_empty() {
            return None;
        }
        return Some(R::merge_string(StringLiteral::boxed(
            SPAN,
            Str::from_in(&class_name, ast.allocator()),
            None,
            ast,
        )));
    }
    if class_name.is_empty() && dynamic.len() == 1 {
        Some(dynamic.remove(0))
    } else {
        let head: &str = if class_name.is_empty() {
            ""
        } else {
            class_name.push(' ');
            class_name.as_str()
        };
        let mut quasis = oxc_allocator::Vec::new_in(ast);
        for index in 0..=dynamic.len() {
            let tail = index == dynamic.len();
            let value = TemplateElementValue {
                raw: Str::from_in(
                    if index == 0 {
                        head
                    } else if tail {
                        ""
                    } else {
                        " "
                    },
                    ast.allocator(),
                ),
                cooked: None,
            };
            quasis.push(TemplateElement::new(SPAN, value, tail, ast));
        }
        Some(R::merge_template(TemplateLiteral::boxed(
            SPAN,
            quasis,
            oxc_allocator::Vec::from_iter_in(
                dynamic
                    .into_iter()
                    .map(ClassMergeRoot::into_merge_expression),
                ast,
            ),
            ast,
        )))
    }
}
