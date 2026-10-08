use oxc_allocator::{Address, CloneIn, GetAddress, GetAllocator};
use oxc_ast::{
    ast::{
        BindingPattern, CallExpression, Declaration, ExportDeclaration, Expression,
        ImportDeclaration, ImportDeclarationSpecifier, Program, TaggedTemplateExpression,
        VariableDeclaration, VariableDeclarationKind,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::{reference::ReferenceId, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

struct Callback<'a> {
    expression: Expression<'a>,
    constant: bool,
}

pub(super) struct Rewritten {
    pub captures: FxHashSet<SymbolId>,
    pub errors: Vec<(u32, String)>,
    pub sites: Vec<(Span, Address)>,
}

struct Sites<'s, 'a> {
    builder: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    styled: FxHashSet<SymbolId>,
    namespaces: FxHashSet<SymbolId>,
    package: &'s str,
    callbacks: FxHashMap<SymbolId, Callback<'a>>,
    reads: FxHashMap<SymbolId, Vec<(ReferenceId, u32)>>,
}

impl<'a> Sites<'_, 'a> {
    fn root(&self, expression: &Expression<'a>) -> bool {
        match crate::utils::unwrap_syntax_only(expression) {
            Expression::Identifier(identifier) => identifier
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id())
                .is_some_and(|symbol| self.styled.contains(&symbol)),
            Expression::StaticMemberExpression(member) => {
                self.root(&member.object)
                    || (member.property.name == "styled"
                        && matches!(&member.object, Expression::Identifier(identifier)
                        if identifier.reference_id.get()
                            .and_then(|id| self.scoping.get_reference(id).symbol_id())
                            .is_some_and(|symbol| self.namespaces.contains(&symbol))))
            }
            Expression::CallExpression(call) => self.root(&call.callee),
            _ => false,
        }
    }

    fn read(&mut self, expression: &Expression<'a>) {
        if let Expression::Identifier(identifier) = crate::utils::unwrap_syntax_only(expression)
            && let Some(reference) = identifier.reference_id.get()
            && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
        {
            self.reads
                .entry(symbol)
                .or_default()
                .push((reference, identifier.span.start));
        }
    }
}

impl<'a> Visit<'a> for Sites<'_, 'a> {
    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        if !import.source.value.starts_with(self.package) {
            return;
        }
        for specifier in import.specifiers.iter().flatten() {
            if let Some(symbol) = specifier.local().symbol_id.get() {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier)
                        if specifier.imported.name() == "styled" =>
                    {
                        self.styled.insert(symbol);
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_)
                    | ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        self.namespaces.insert(symbol);
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(_) => {}
                }
            }
        }
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(binding) = &declarator.id
                && let Some(init) = &declarator.init
                && let Some(symbol) = binding.symbol_id.get()
                && matches!(
                    crate::utils::unwrap_syntax_only(init),
                    Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                )
            {
                self.callbacks.insert(
                    symbol,
                    Callback {
                        expression: crate::utils::unwrap_syntax_only(init)
                            .clone_in_with_semantic_ids(self.builder.allocator()),
                        constant: declaration.kind == VariableDeclarationKind::Const,
                    },
                );
            }
        }
        walk::walk_variable_declaration(self, declaration);
    }

    fn visit_export_declaration(&mut self, export: &ExportDeclaration<'a>) {
        walk::walk_export_declaration(self, export);
        if let Declaration::VariableDeclaration(declaration) = &export.declaration {
            for declarator in &declaration.declarations {
                if let BindingPattern::BindingIdentifier(binding) = &declarator.id
                    && let Some(callback) = binding
                        .symbol_id
                        .get()
                        .and_then(|symbol| self.callbacks.get_mut(&symbol))
                {
                    callback.constant = false;
                }
            }
        }
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.root(&call.callee) {
            let start = match crate::utils::unwrap_syntax_only(&call.callee) {
                Expression::Identifier(_) => Some(1),
                Expression::StaticMemberExpression(member)
                    if matches!(member.property.name.as_str(), "attrs" | "withConfig") =>
                {
                    None
                }
                _ => Some(0),
            };
            if let Some(start) = start {
                for argument in call.arguments.iter().skip(start) {
                    if let Some(expression) = argument.as_expression() {
                        self.read(expression);
                    }
                }
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_tagged_template_expression(&mut self, tagged: &TaggedTemplateExpression<'a>) {
        if self.root(&tagged.tag) {
            for expression in &tagged.quasi.expressions {
                self.read(expression);
            }
        }
        walk::walk_tagged_template_expression(self, tagged);
    }
}

/// Clone read-only callbacks only at styled rule sites; preserve semantic IDs
/// so captures can be renamed before lowering turns references into source.
pub(super) fn rewrite<'a>(
    builder: &AstBuilder<'a>,
    program: &mut Program<'a>,
    scoping: &Scoping,
    package: &str,
) -> Rewritten {
    let mut sites = Sites {
        builder,
        scoping,
        package,
        styled: FxHashSet::default(),
        namespaces: FxHashSet::default(),
        callbacks: FxHashMap::default(),
        reads: FxHashMap::default(),
    };
    sites.visit_program(program);
    let deferred = super::local_capture_reads::deferred_reads(program, scoping);
    let mut replacements = FxHashMap::default();
    let mut captures = FxHashSet::default();
    let mut errors = Vec::new();
    for (symbol, callback) in &sites.callbacks {
        let Some(reads) = sites.reads.get(symbol) else {
            continue;
        };
        let references = scoping.get_resolved_reference_ids(*symbol);
        let safe = callback.constant
            && references.len() == reads.len()
            && references
                .iter()
                .all(|id| !scoping.get_reference(*id).is_write())
            && reads.iter().all(|(id, start)| {
                deferred.contains(id) || *start >= callback.expression.span().end
            });
        if !safe {
            errors.push((
                scoping.symbol_span(*symbol).start,
                crate::utils::local_style_error(scoping.symbol_name(*symbol)),
            ));
            continue;
        }
        let Some(captured) = super::bound_callback_captures::collect(&callback.expression, scoping)
        else {
            errors.push((
                scoping.symbol_span(*symbol).start,
                crate::utils::local_style_error(scoping.symbol_name(*symbol)),
            ));
            continue;
        };
        captures.extend(captured);
        for (reference, _) in reads {
            replacements.insert(*reference, &callback.expression);
        }
    }
    let mut rewrite = Rewrite {
        builder,
        replacements,
        sites: Vec::new(),
    };
    rewrite.visit_program(program);
    errors.sort_unstable_by_key(|(start, _)| *start);
    Rewritten {
        captures,
        errors,
        sites: rewrite.sites,
    }
}

struct Rewrite<'s, 'a> {
    builder: &'s AstBuilder<'a>,
    replacements: FxHashMap<ReferenceId, &'s Expression<'a>>,
    sites: Vec<(Span, Address)>,
}

impl<'a> VisitMut<'a> for Rewrite<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Expression::Identifier(identifier) = expression
            && let Some(callback) = identifier
                .reference_id
                .get()
                .and_then(|id| self.replacements.get(&id))
        {
            let span = identifier.span;
            *expression = callback.clone_in_with_semantic_ids(self.builder.allocator());
            self.sites.push((span, expression.address()));
            return;
        }
        walk_mut::walk_expression(self, expression);
    }
}
