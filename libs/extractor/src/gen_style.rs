use crate::ExtractStyleProp;
use crate::extract_style::style_property::StyleProperty;
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
    StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use std::collections::BTreeMap;
pub fn gen_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &[ExtractStyleProp<'a>],
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    if style_props.is_empty() {
        return None;
    }
    let mut properties: Vec<_> = Vec::with_capacity(style_props.len());
    properties.extend(
        style_props
            .iter()
            .flat_map(|style| gen_style(ast_builder, style, filename))
            .rev(),
    );
    if properties.is_empty() {
        return None;
    }
    Some(Expression::new_object_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(properties, ast_builder),
        ast_builder,
    ))
}
/// Push `cond ? value : undefined` (`value_when_true`) or `cond ? undefined : value`
/// object properties for every property generated from `styles`.
fn push_one_sided_conditional<'a>(
    ast_builder: &AstBuilder<'a>,
    properties: &mut Vec<ObjectPropertyKind<'a>>,
    condition: &Expression<'a>,
    styles: &ExtractStyleProp<'a>,
    filename: Option<&str>,
    value_when_true: bool,
) {
    for p in gen_style(ast_builder, styles, filename) {
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
}

fn gen_style<'a>(
    ast_builder: &AstBuilder<'a>,
    style: &ExtractStyleProp<'a>,
    filename: Option<&str>,
) -> Vec<ObjectPropertyKind<'a>> {
    let mut properties = vec![];
    if let ExtractStyleProp::Static(st) = style {
        if let Some(StyleProperty::Variable {
            variable_name,
            identifier,
            ..
        }) = st.extract(filename)
        {
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    SPAN,
                    Str::from_in(&variable_name, ast_builder.allocator()),
                    None,
                    ast_builder,
                )),
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(&identifier, ast_builder.allocator()),
                    ast_builder,
                ),
                false,
                false,
                false,
                ast_builder,
            ));
        }
    } else if let ExtractStyleProp::StaticArray(res) = style {
        properties.extend(
            res.iter()
                .flat_map(|r| gen_style(ast_builder, r, filename))
                .rev(),
        );
    } else if let ExtractStyleProp::Conditional {
        condition,
        consequent,
        alternate,
    } = style
    {
        let r = (consequent, alternate);
        if let (None, Some(c)) = r {
            push_one_sided_conditional(ast_builder, &mut properties, condition, c, filename, false);
        } else if let (Some(c), None) = r {
            push_one_sided_conditional(ast_builder, &mut properties, condition, c, filename, true);
        } else if let (Some(c), Some(a)) = r {
            let collect_c = gen_style(ast_builder, c, filename);
            let collect_a = gen_style(ast_builder, a, filename);
            if collect_c.is_empty() && collect_a.is_empty() {
                return vec![];
            }
            // Index the alternate branch by key ONCE so the two-sided merge is O(C+A)
            // instead of the old O(C*A) nested `iter().any(..)` key comparisons. The map
            // stores each key's FIRST index in `collect_a` (only inserting on the first
            // sight of a key), exactly reproducing the previous "first matching q wins"
            // ternary. `key.name()` is a borrowed `Cow<str>` for the common static-ident
            // props, so keys are cheap to hash and don't allocate.
            let mut a_by_key: FxHashMap<std::borrow::Cow<str>, usize> =
                FxHashMap::with_capacity_and_hasher(collect_a.len(), FxBuildHasher);
            for (j, q) in collect_a.iter().enumerate() {
                if let ObjectPropertyKind::ObjectProperty(q) = q
                    && let Some(name) = q.key.name()
                {
                    a_by_key.entry(name).or_insert(j);
                }
            }
            // Set of consequent keys: an alternate entry is emitted alone (a-side pass
            // below) iff NO consequent shares its key — identical to the old
            // `collect_c.iter().any(..)` probe, now a single O(1) set lookup.
            let mut c_keys: FxHashSet<std::borrow::Cow<str>> =
                FxHashSet::with_capacity_and_hasher(collect_c.len(), FxBuildHasher);

            for p in &collect_c {
                let matched = if let ObjectPropertyKind::ObjectProperty(p) = p
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
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
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
                // Emit an alternate-only property when it has no consequent counterpart:
                // either its key is absent from `c_keys`, or it has no resolvable key at
                // all (the old probe's `matches!` never matched such an entry either).
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
    } else if let ExtractStyleProp::MemberExpression { map, expression } = style {
        let mut tmp_map = BTreeMap::<String, Vec<(String, String)>>::new();
        for (key, value) in map {
            for style in value.extract() {
                if let Some(StyleProperty::Variable {
                    variable_name,
                    identifier,
                    ..
                }) = style.extract(filename)
                {
                    tmp_map
                        .entry(variable_name)
                        .or_default()
                        .push((key.clone(), identifier));
                }
            }
        }

        for (key, value) in tmp_map {
            let v = if value.len() == 1 {
                // do not create object expression when property is single
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(&value[0].1, ast_builder.allocator()),
                    ast_builder,
                )
            } else {
                Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                    SPAN,
                    Expression::new_object_expression(
                        SPAN,
                        oxc_allocator::Vec::from_iter_in(
                            value.into_iter().map(|(k, v)| {
                                ObjectPropertyKind::new_object_property(
                                    SPAN,
                                    PropertyKind::Init,
                                    PropertyKey::new_static_identifier(
                                        SPAN,
                                        Str::from_in(&k, ast_builder.allocator()),
                                        ast_builder,
                                    ),
                                    Expression::new_identifier(
                                        SPAN,
                                        Str::from_in(&v, ast_builder.allocator()),
                                        ast_builder,
                                    ),
                                    false,
                                    false,
                                    false,
                                    ast_builder,
                                )
                            }),
                            ast_builder,
                        ),
                        ast_builder,
                    ),
                    expression.clone_in(ast_builder.allocator()),
                    false,
                    ast_builder,
                ))
            };
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    SPAN,
                    Str::from_in(&key, ast_builder.allocator()),
                    None,
                    ast_builder,
                )),
                v,
                false,
                false,
                false,
                ast_builder,
            ));
        }
    }
    // Cache each property's key once (`PropertyKey::name()` may allocate for computed
    // keys) instead of recomputing it twice per comparison. The cached key stays a
    // borrowed `Cow<str>` for the common `StaticIdentifier` props (`color`, `padding`,
    // ...), so sorting no longer heap-allocates an owned `String` per property.
    // `Cow<str>: Ord` compares by contents, and `Reverse` keeps the existing descending
    // order, so the generated property order is byte-identical.
    properties.sort_by_cached_key(|p| std::cmp::Reverse(object_property_key(p)));
    properties
}

fn object_property_key<'k>(p: &ObjectPropertyKind<'k>) -> Option<std::borrow::Cow<'k, str>> {
    if let ObjectPropertyKind::ObjectProperty(p) = p {
        p.key.name()
    } else {
        None
    }
}
