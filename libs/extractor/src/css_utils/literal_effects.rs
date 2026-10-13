use super::{cursor, literal::CssText};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{ObjectPropertyKind, PropertyKey, PropertyKind, StringLiteral};
use oxc_ast::builder::AstBuilder;
use oxc_span::GetSpan;
use std::ops::Range;

impl<'a> CssText<'a> {
    pub(super) fn visible(&self, at: usize) -> bool {
        !cursor::comments(&self.text)
            .iter()
            .any(|range| range.contains(&at))
    }

    pub(super) fn comment_effects(
        &self,
        ast: &AstBuilder<'a>,
        range: Range<usize>,
        props: &mut oxc_allocator::Vec<'a, ObjectPropertyKind<'a>>,
    ) {
        let bodies: Vec<_> = cursor::items(&self.text[range.clone()])
            .into_iter()
            .filter_map(|item| match item {
                cursor::Item::Block { body, .. } => {
                    Some(body.start + range.start..body.end + range.start)
                }
                cursor::Item::Declaration { .. } | cursor::Item::Statement(_) => None,
            })
            .collect();
        for (hole, expression) in &self.holes {
            if !range.contains(&hole.start)
                || self.visible(hole.start)
                || bodies.iter().any(|body| body.contains(&hole.start))
            {
                continue;
            }
            props.push(ObjectPropertyKind::new_object_property(
                expression.span(),
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    expression.span(),
                    "__devupLiteralEffect",
                    None,
                    ast,
                )),
                expression.clone_in_with_semantic_ids(ast.allocator()),
                false,
                false,
                false,
                ast,
            ));
        }
        props.sort_by_key(|property| property.span().start);
    }
}
