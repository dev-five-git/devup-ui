use super::{
    cursor::{self, Item},
    literal::CssText,
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey, PropertyKind, StringLiteral};
use oxc_ast::builder::AstBuilder;
use std::ops::Range;

impl<'a> CssText<'a> {
    pub(crate) fn object(&self, ast: &AstBuilder<'a>, range: Range<usize>) -> Expression<'a> {
        self.scoped_object(ast, range, false)
    }

    pub(crate) fn key(&self, range: Range<usize>) -> Result<String, &Expression<'a>> {
        let mut name = String::new();
        let mut from = range.start;
        for (hole, expression) in self
            .holes
            .iter()
            .filter(|(hole, _)| hole.start >= range.start && hole.end <= range.end)
        {
            name.push_str(&self.text[from..hole.start]);
            let Some(value) = crate::utils::get_string_by_literal_expression(expression) else {
                return Err(expression);
            };
            name.push_str(&value);
            from = hole.end;
        }
        name.push_str(&self.text[from..range.end]);
        Ok(super::literal_values::canonical_key(
            cursor::clean(&name).trim(),
        ))
    }

    pub(crate) fn has_order(&self, range: Range<usize>) -> bool {
        cursor::items(&self.text[range.clone()])
            .iter()
            .any(|item| match item {
                Item::Declaration { key, .. } => self
                    .key(key.start + range.start..key.end + range.start)
                    .is_ok_and(|name| crate::style_order::reserved(&name)),
                Item::Block { body, .. } => {
                    self.has_order(body.start + range.start..body.end + range.start)
                }
                Item::Statement(_) => false,
            })
    }

    pub(crate) fn scoped_object(
        &self,
        ast: &AstBuilder<'a>,
        range: Range<usize>,
        global: bool,
    ) -> Expression<'a> {
        let mut props = oxc_allocator::Vec::new_in(ast);
        for item in cursor::items(&self.text[range.clone()]) {
            let shifted = |part: Range<usize>| part.start + range.start..part.end + range.start;
            let (key, value, block) = match item {
                Item::Declaration { key, value } => {
                    let key = shifted(key);
                    let name = self.key(key.clone()).unwrap_or_default();
                    (
                        key,
                        self.value(ast, shifted(value), crate::style_order::reserved(&name)),
                        false,
                    )
                }
                Item::Block { prelude, body } => {
                    let key = shifted(prelude);
                    let grouping = global && self.text[key.clone()].trim().starts_with('@');
                    (key, self.scoped_object(ast, shifted(body), grouping), true)
                }
                Item::Statement(range) => {
                    let key = shifted(range);
                    if let Some((hole, expression)) = self
                        .holes
                        .iter()
                        .find(|(hole, _)| hole.start >= key.start && hole.end <= key.end)
                    {
                        let name = if *hole == key {
                            "__devupLiteralMixin"
                        } else {
                            "__devupLiteralUnplaced"
                        };
                        props.push(ObjectPropertyKind::new_object_property(
                            self.span(&key),
                            PropertyKind::Init,
                            PropertyKey::StringLiteral(StringLiteral::boxed(
                                self.span(&key),
                                name,
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
                    continue;
                }
            };
            let name = match self.key(key.clone()) {
                Ok(name) => name,
                Err(expression) => {
                    props.push(ObjectPropertyKind::new_object_property(
                        self.span(&key),
                        PropertyKind::Init,
                        PropertyKey::StringLiteral(StringLiteral::boxed(
                            self.span(&key),
                            "__devupLiteralUnplaced",
                            None,
                            ast,
                        )),
                        expression.clone_in_with_semantic_ids(ast.allocator()),
                        false,
                        false,
                        false,
                        ast,
                    ));
                    continue;
                }
            };
            let selector = block && !name.starts_with('@');
            let name = if global || name.starts_with('@') || !block {
                name
            } else {
                super::descendants(&name)
            };
            let property = ObjectPropertyKind::new_object_property(
                self.span(&key),
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    self.span(&key),
                    ast.allocator().alloc_str(&name),
                    None,
                    ast,
                )),
                value,
                false,
                false,
                false,
                ast,
            );
            if selector {
                let mut record = oxc_allocator::Vec::new_in(ast);
                record.push(property);
                props.push(ObjectPropertyKind::new_object_property(
                    self.span(&key),
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(StringLiteral::boxed(
                        self.span(&key),
                        "selectors",
                        None,
                        ast,
                    )),
                    Expression::new_object_expression(self.span(&key), record, ast),
                    false,
                    false,
                    false,
                    ast,
                ));
            } else {
                props.push(property);
            }
        }
        self.comment_effects(ast, range.clone(), &mut props);
        Expression::new_object_expression(self.span(&range), props, ast)
    }
}
