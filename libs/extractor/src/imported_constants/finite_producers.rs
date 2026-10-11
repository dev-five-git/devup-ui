use super::{
    Allocator, AstBuilder, Constant, Expression, FxHashMap, FxHashSet, GetAllocator,
    IdentifierReference, ModuleScope, Modules, Parser, Rc, Scoping, SemanticBuilder, SourceType,
    binding_of, constant_literal,
};
use oxc_allocator::CloneIn;
use oxc_ast_visit::{Visit, VisitMut, walk_mut};

impl ModuleScope<'_, '_> {
    pub(super) fn finite_css(
        &mut self,
        modules: &mut Modules<'_>,
        span: oxc_span::Span,
    ) -> Option<crate::finite_styles::FiniteStyles> {
        if self.finite_origins.is_none() {
            self.finite_origins = Some(FxHashMap::default());
            let source = self.source?;
            let aliased = crate::import_alias_visit::transform_import_aliases_with_edits(
                source,
                self.path,
                &modules.option.package,
                &modules.option.import_aliases,
            );
            let allocator = Allocator::default();
            let builder = AstBuilder::new(&allocator);
            let mut program = Parser::new(
                &allocator,
                &aliased.code,
                SourceType::from_path(self.path).ok()?,
            )
            .parse()
            .program;
            let scoping = Rc::new(
                SemanticBuilder::new()
                    .build(&program)
                    .semantic
                    .into_scoping(),
            );
            let mut roots = Roots {
                scoping: &scoping,
                names: FxHashSet::default(),
            };
            roots.visit_program(&program);
            let mut literals = FxHashMap::default();
            let mut finite = FxHashMap::default();
            let mut known = FxHashMap::default();
            let mut arrays = FxHashMap::default();
            for name in roots.names {
                if self.is_style_import(modules.option, &name) {
                    continue;
                }
                if let Some(value) = self.lookup(modules, &name) {
                    transport(&name, &value, &mut finite);
                    transport_arrays(&name, &value, &mut arrays);
                    if let Constant::Style(Some(styles)) = &value {
                        known.insert(name.clone(), styles.as_ref().clone());
                    }
                    if let Some(literal) = constant_literal(&builder, &value, true) {
                        literals.insert(name, literal);
                    }
                }
            }
            Replace {
                builder: &builder,
                scoping: &scoping,
                literals,
            }
            .visit_program(&mut program);
            let mut visitor = crate::visit::DevupVisitor::new(
                &allocator,
                self.path,
                &modules.option.package,
                vec![],
                crate::css_bucket(self.path, modules.option),
            );
            visitor.import_css(known);
            visitor.import_finite_css(finite);
            visitor.import_css_arrays(arrays);
            visitor.reuse_scoping(Some(scoping));
            visitor.visit_program(&mut program);
            if visitor.errors.is_empty() && visitor.unknown_parts.is_empty() {
                self.finite_arrays = visitor
                    .finite_array_origins()
                    .into_iter()
                    .map(|((start, end), styles)| {
                        Some((
                            (
                                crate::import_alias_visit::source_offset(
                                    &aliased.edits,
                                    usize::try_from(start).ok()?,
                                ),
                                crate::import_alias_visit::source_offset(
                                    &aliased.edits,
                                    usize::try_from(end).ok()?,
                                ),
                            ),
                            styles,
                        ))
                    })
                    .collect::<Option<_>>()?;
                self.finite_origins = visitor
                    .finite_origins()
                    .into_iter()
                    .map(|((start, end), styles)| {
                        Some((
                            (
                                crate::import_alias_visit::source_offset(
                                    &aliased.edits,
                                    usize::try_from(start).ok()?,
                                ),
                                crate::import_alias_visit::source_offset(
                                    &aliased.edits,
                                    usize::try_from(end).ok()?,
                                ),
                            ),
                            styles,
                        ))
                    })
                    .collect();
            }
        }
        self.finite_origins
            .as_ref()?
            .get(&(
                usize::try_from(span.start).ok()?,
                usize::try_from(span.end).ok()?,
            ))
            .cloned()
    }
    pub(super) fn finite_array(
        &self,
        span: oxc_span::Span,
    ) -> Option<Vec<crate::finite_styles::FiniteStyles>> {
        self.finite_arrays
            .get(&(
                usize::try_from(span.start).ok()?,
                usize::try_from(span.end).ok()?,
            ))
            .cloned()
    }
}

pub(super) fn transport(
    name: &str,
    value: &Constant,
    finite: &mut FxHashMap<String, crate::finite_styles::FiniteStyles>,
) {
    match value {
        Constant::FiniteStyle(styles) => {
            finite.insert(name.to_string(), styles.clone());
        }
        Constant::Object(values) => {
            for (key, value) in values.iter() {
                transport(&format!("{name}.{key}"), value, finite);
            }
        }
        Constant::Record(values) => {
            for (key, value) in values.iter() {
                transport(&format!("{name}.{key}"), value, finite);
            }
        }
        Constant::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                transport(&format!("{name}.{index}"), value, finite);
            }
        }
        _ => {}
    }
}

pub(super) fn transport_arrays(
    name: &str,
    value: &Constant,
    arrays: &mut FxHashMap<String, Vec<crate::finite_styles::FiniteStyles>>,
) {
    match value {
        Constant::Array(values) => {
            if let Some(finite) = values
                .iter()
                .map(|value| match value {
                    Constant::FiniteStyle(finite) => Some(finite.clone()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            {
                arrays.insert(name.to_string(), finite);
            }
        }
        Constant::Object(values) => {
            for (key, value) in values.iter() {
                transport_arrays(&format!("{name}.{key}"), value, arrays);
            }
        }
        Constant::Record(values) => {
            for (key, value) in values.iter() {
                transport_arrays(&format!("{name}.{key}"), value, arrays);
            }
        }
        _ => {}
    }
}

struct Roots<'s> {
    scoping: &'s Scoping,
    names: FxHashSet<String>,
}
impl<'a> Visit<'a> for Roots<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if let Some(symbol) = binding_of(self.scoping, identifier)
            && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
        {
            self.names.insert(identifier.name.to_string());
        }
    }
}

struct Replace<'b, 'a> {
    builder: &'b AstBuilder<'a>,
    scoping: &'b Scoping,
    literals: FxHashMap<String, Expression<'a>>,
}
impl<'a> VisitMut<'a> for Replace<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Expression::Identifier(identifier) = expression
            && binding_of(self.scoping, identifier).is_some_and(|symbol| {
                self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            })
            && let Some(literal) = self.literals.get(identifier.name.as_str())
        {
            *expression = literal.clone_in(self.builder.allocator());
        } else {
            walk_mut::walk_expression(self, expression);
        }
    }
}
