use super::{ExtractStyleProp, gen_style};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectPropertyKind, PropertyKind},
    builder::AstBuilder,
};
use oxc_span::SPAN;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

pub(super) struct InlineProjection<'b, 'a> {
    pub(super) ast: &'b AstBuilder<'a>,
    pub(super) filename: Option<&'b str>,
}

impl<'a> InlineProjection<'_, 'a> {
    /// Extend with the same lazy, reversed leaf-generation schedule at every depth.
    pub(super) fn extend_reversed(
        &self,
        properties: &mut Vec<ObjectPropertyKind<'a>>,
        styles: &[ExtractStyleProp<'a>],
    ) {
        properties.extend(
            styles
                .iter()
                .flat_map(|style| gen_style(self.ast, style, self.filename))
                .rev(),
        );
    }

    pub(super) fn styles(&self, style_props: &[ExtractStyleProp<'a>]) -> Option<Expression<'a>> {
        let ast_builder = self.ast;
        if style_props.is_empty() {
            return None;
        }
        let mut properties: Vec<_> = Vec::with_capacity(style_props.len());
        self.extend_reversed(&mut properties, style_props);
        let mut assigned = FxHashSet::default();
        properties.retain(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => property
                .key
                .name()
                .is_none_or(|key| assigned.insert(key.into_owned())),
            ObjectPropertyKind::SpreadProperty(_) => true,
        });
        if properties.is_empty() {
            return None;
        }
        Some(Expression::new_object_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(properties, ast_builder),
            ast_builder,
        ))
    }

    fn one_sided(
        &self,
        condition: &Expression<'a>,
        branch: (&ExtractStyleProp<'a>, bool),
    ) -> Vec<ObjectPropertyKind<'a>> {
        let ast_builder = self.ast;
        let (styles, value_when_true) = branch;
        let mut properties = Vec::new();
        for p in gen_style(ast_builder, styles, self.filename) {
            if let ObjectPropertyKind::ObjectProperty(p) = p {
                let value = p.value.clone_in(ast_builder.allocator());
                let undefined = Expression::new_identifier(SPAN, "undefined", ast_builder);
                let (consequent, alternate) = if value_when_true {
                    (value, undefined)
                } else {
                    (undefined, value)
                };
                properties.push(ObjectPropertyKind::new_object_property(
                    SPAN,
                    PropertyKind::Init,
                    p.key.clone_in(ast_builder.allocator()),
                    Expression::new_conditional_expression(
                        SPAN,
                        condition.clone_in(ast_builder.allocator()),
                        consequent,
                        alternate,
                        ast_builder,
                    ),
                    false,
                    false,
                    false,
                    ast_builder,
                ));
            }
        }
        properties
    }

    pub(super) fn conditional(
        &self,
        condition: &Expression<'a>,
        branches: (Option<&ExtractStyleProp<'a>>, Option<&ExtractStyleProp<'a>>),
    ) -> Vec<ObjectPropertyKind<'a>> {
        let ast_builder = self.ast;
        let mut properties = Vec::new();
        match branches {
            (None, Some(c)) => return self.one_sided(condition, (c, false)),
            (Some(c), None) => return self.one_sided(condition, (c, true)),
            (None, None) => {}
            (Some(c), Some(a)) => {
                let collect_c = gen_style(ast_builder, c, self.filename);
                let collect_a = gen_style(ast_builder, a, self.filename);
                if collect_c.is_empty() && collect_a.is_empty() {
                    return vec![];
                }
                // Keep the first alternate entry for each key, as before the split.
                let mut a_by_key: FxHashMap<std::borrow::Cow<str>, usize> =
                    FxHashMap::with_capacity_and_hasher(collect_a.len(), FxBuildHasher);
                for (j, q) in collect_a.iter().enumerate() {
                    if let ObjectPropertyKind::ObjectProperty(q) = q
                        && let Some(name) = q.key.name()
                    {
                        a_by_key.entry(name).or_insert(j);
                    }
                }
                let mut c_keys: FxHashSet<std::borrow::Cow<str>> =
                    FxHashSet::with_capacity_and_hasher(collect_c.len(), FxBuildHasher);
                for p in &collect_c {
                    let mut matched = false;
                    if let ObjectPropertyKind::ObjectProperty(p) = p
                        && let Some(name) = p.key.name()
                    {
                        c_keys.insert(name.clone());
                        if let Some(&j) = a_by_key.get(&name)
                            && let ObjectPropertyKind::ObjectProperty(q) = &collect_a[j]
                        {
                            properties.push(ObjectPropertyKind::new_object_property(
                                SPAN,
                                PropertyKind::Init,
                                p.key.clone_in(ast_builder.allocator()),
                                Expression::new_conditional_expression(
                                    SPAN,
                                    condition.clone_in(ast_builder.allocator()),
                                    p.value.clone_in(ast_builder.allocator()),
                                    q.value.clone_in(ast_builder.allocator()),
                                    ast_builder,
                                ),
                                false,
                                false,
                                false,
                                ast_builder,
                            ));
                            matched = true;
                        }
                    }
                    if !matched && let ObjectPropertyKind::ObjectProperty(p) = p {
                        properties.push(ObjectPropertyKind::new_object_property(
                            SPAN,
                            PropertyKind::Init,
                            p.key.clone_in(ast_builder.allocator()),
                            p.value.clone_in(ast_builder.allocator()),
                            false,
                            false,
                            false,
                            ast_builder,
                        ));
                    }
                }
                for q in &collect_a {
                    let unmatched = if let ObjectPropertyKind::ObjectProperty(qq) = q {
                        qq.key.name().is_none_or(|name| !c_keys.contains(&name))
                    } else {
                        false
                    };
                    if unmatched && let ObjectPropertyKind::ObjectProperty(q) = q {
                        properties.push(ObjectPropertyKind::new_object_property(
                            SPAN,
                            PropertyKind::Init,
                            q.key.clone_in(ast_builder.allocator()),
                            q.value.clone_in(ast_builder.allocator()),
                            false,
                            false,
                            false,
                            ast_builder,
                        ));
                    }
                }
            }
        }
        properties
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[serial_test::serial]
    fn inline_protocol_emits_no_variables_when_both_conditional_branches_are_absent() {
        // Given: an actual parsed condition and the public compiler IR's absent branches.
        crate::compiler_policy::tests::reset();
        let before = (
            css::class_map::get_class_map(),
            css::file_map::get_file_map(),
            css::file_map::get_original_ids(),
            css::file_map::get_canonical_map(),
        );
        let allocator = oxc_allocator::Allocator::default();
        let condition = oxc_parser::Parser::new(&allocator, "flag", oxc_span::SourceType::tsx())
            .parse_expression();
        assert!(condition.is_ok());
        condition.into_iter().for_each(|condition| {
            let styles = [crate::ExtractStyleProp::Conditional {
                condition,
                consequent: None,
                alternate: None,
            }];
            let ast = oxc_ast::builder::AstBuilder::new(&allocator);
            // When: the real inline compiler projects the typed empty conditional.
            let generated = crate::gen_style::gen_styles(&ast, &styles, Some("coverage.tsx"));
            // Then: neither an empty style object nor a variable/name allocation is invented.
            assert!(generated.is_none());
            assert_eq!(
                (
                    css::class_map::get_class_map(),
                    css::file_map::get_file_map(),
                    css::file_map::get_original_ids(),
                    css::file_map::get_canonical_map()
                ),
                before
            );
        });
    }
}
