//! What a module binds to Devup UI under another name, and the style
//! constants it reads before they are declared

use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Program, Statement,
    StaticMemberExpression, VariableDeclaration, VariableDeclarationKind,
};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::{Bound, Namespace, Rewriter, compiled_export, export_symbol, reference_symbol};

/// Where a binding leads: the module it is read from, and the name read, `None`
/// for the module's namespace
pub(super) type Reach = (String, Option<String>);

/// The statement exporting `exported` as what `reach` names, from the module
/// the binding was imported from
fn reexport(exported: &str, reach: &Reach) -> String {
    let quoted = serde_json::to_string(&reach.0).unwrap_or_default();
    match &reach.1 {
        Some(imported) => format!("export {{ {imported} as {exported} }} from {quoted};"),
        None => format!("export * as {exported} from {quoted};"),
    }
}

/// The text of `declaration`'s declarator at `span` removed from it: the
/// whole declaration when it is the only one
pub(super) fn declarator_removal(declaration: &VariableDeclaration<'_>, span: Span) -> (u32, u32) {
    let declarators = &declaration.declarations;
    let index = declarators
        .iter()
        .position(|other| other.span == span)
        .unwrap_or_default();
    if declarators.len() == 1 {
        (declaration.span.start, declaration.span.end)
    } else if index + 1 < declarators.len() {
        (span.start, declarators[index + 1].span.start)
    } else {
        (declarators[index - 1].span.end, span.end)
    }
}

/// Whether `name` is the value of an assignment in `code`, as in `= name`
pub(super) fn is_assigned_from(code: &str, name: &str) -> bool {
    code.match_indices(name).any(|(index, _)| {
        code[..index].trim_end().ends_with('=')
            && !code[index + name.len()..]
                .starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '$')
    })
}

/// The edits that export Devup UI from a module as a re-export of the module
/// it came from: `export { Box }` after an import, `export default Box`,
/// `export const B = Box`. A module compiled with the import removed would
/// otherwise export a name nothing declares.
pub(super) fn export_edits(
    semantic: &Semantic<'_>,
    code: &str,
    mut locals: FxHashMap<SymbolId, Reach>,
) -> Vec<(usize, usize, String)> {
    let mut edits = Vec::new();
    let program = semantic.nodes().program();
    for statement in &program.body {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                alias_reaches(declaration, &mut locals, semantic);
            }
            Statement::ExportDeclaration(export) => {
                if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                    let aliases = alias_reaches(declaration, &mut locals, semantic);
                    if !aliases.is_empty() {
                        let mut statements: Vec<String> = Vec::new();
                        let others: Vec<&str> = declaration
                            .declarations
                            .iter()
                            .flat_map(|declarator| declarator.id.get_binding_identifiers())
                            .map(|id| id.name.as_str())
                            .filter(|name| aliases.iter().all(|(alias, _)| alias != name))
                            .collect();
                        if !others.is_empty() {
                            statements.push(format!("export {{ {} }};", others.join(", ")));
                        }
                        statements
                            .extend(aliases.iter().map(|(name, reach)| reexport(name, reach)));
                        edits.push((
                            export.span.start as usize,
                            declaration.span.start as usize,
                            String::new(),
                        ));
                        edits.push((
                            export.span.end as usize,
                            export.span.end as usize,
                            format!("\n{}", statements.join("\n")),
                        ));
                    }
                }
            }
            Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => {
                let mut statements = Vec::new();
                let mut kept = Vec::new();
                for specifier in &export.specifiers {
                    let reach = export_symbol(&specifier.local, semantic)
                        .and_then(|symbol| locals.get(&symbol));
                    match reach {
                        Some(reach) if !specifier.export_kind.is_type() => {
                            statements.push(reexport(specifier.exported.name().as_str(), reach));
                        }
                        _ => kept.push(
                            &code[specifier.span.start as usize..specifier.span.end as usize],
                        ),
                    }
                }
                if !statements.is_empty() {
                    if !kept.is_empty() {
                        statements.insert(0, format!("export {{ {} }};", kept.join(", ")));
                    }
                    edits.push((
                        export.span.start as usize,
                        export.span.end as usize,
                        statements.join("\n"),
                    ));
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                if let ExportDefaultDeclarationKind::Identifier(name) = &export.declaration
                    && let Some(reach) =
                        reference_symbol(name, semantic).and_then(|symbol| locals.get(&symbol))
                    && reach.1.is_some()
                {
                    edits.push((
                        export.span.start as usize,
                        export.span.end as usize,
                        reexport("default", reach),
                    ));
                }
            }
            _ => {}
        }
    }
    edits
}

/// Where `Namespace.member` leads, when `Namespace` is a local of Devup UI and
/// the build compiles `member` away
fn namespace_member_reach(
    member: &StaticMemberExpression<'_>,
    locals: &FxHashMap<SymbolId, Reach>,
    semantic: &Semantic<'_>,
) -> Option<Reach> {
    if let Expression::Identifier(object) = &member.object
        && let Some((source, None)) =
            reference_symbol(object, semantic).and_then(|symbol| locals.get(&symbol))
        && compiled_export(member.property.name.as_str())
    {
        Some((source.clone(), Some(member.property.name.to_string())))
    } else {
        None
    }
}

/// The `const` bindings of `declaration` that alias a local of Devup UI or a
/// member of its namespace, `(name, where it leads)`, added to `locals`
fn alias_reaches<'a>(
    declaration: &VariableDeclaration<'a>,
    locals: &mut FxHashMap<SymbolId, Reach>,
    semantic: &Semantic<'_>,
) -> Vec<(&'a str, Reach)> {
    let mut aliases = Vec::new();
    if declaration.kind != VariableDeclarationKind::Const {
        return aliases;
    }
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            continue;
        };
        let reach = match &declarator.init {
            Some(Expression::Identifier(init)) => reference_symbol(init, semantic)
                .and_then(|symbol| locals.get(&symbol))
                .cloned(),
            Some(Expression::StaticMemberExpression(member)) => {
                namespace_member_reach(member, locals, semantic)
            }
            _ => None,
        };
        if let Some(reach) = reach {
            locals.extend(id.symbol_id.get().map(|symbol| (symbol, reach.clone())));
            aliases.push((id.name.as_str(), reach));
        }
    }
    aliases
}

/// The style functions whose results a module may read before it declares them
const HOISTED: [&str; 2] = ["css", "keyframes"];

impl Rewriter<'_, '_, '_, '_> {
    fn is_deferred(&self, node: NodeId) -> bool {
        self.semantic.nodes().ancestor_kinds(node).any(|kind| {
            matches!(
                kind,
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
            )
        })
    }

    /// Follows what the module aliases: `const c = css` in a function reads
    /// the package where it is used; `const D = Devup` is another namespace
    pub(super) fn follow_aliases(
        &mut self,
        bound: &[Bound<'_, '_>],
        namespaces: &mut Vec<Namespace>,
    ) {
        let scoping = self.semantic.scoping();
        let nodes = self.semantic.nodes();
        let mut named: FxHashMap<SymbolId, (&Bound<'_, '_>, Reach)> = bound
            .iter()
            .filter_map(|bound| {
                Some((bound.binding.symbol_id.get()?, (bound, bound.devup.clone())))
            })
            .collect();
        let first_new = namespaces.len();
        self.namespace_aliases(namespaces);
        let spaces: FxHashMap<SymbolId, usize> = namespaces
            .iter()
            .enumerate()
            .filter_map(|(index, namespace)| Some((namespace.symbol?, index)))
            .collect();
        for node in nodes.iter() {
            let AstKind::VariableDeclarator(declarator) = node.kind() else {
                continue;
            };
            let (BindingPattern::BindingIdentifier(id), Some(Expression::Identifier(init))) =
                (&declarator.id, &declarator.init)
            else {
                continue;
            };
            let declaration_id = nodes.parent_id(node.id());
            let (Some(declaration), Some(alias), Some(origin)) = (
                nodes.kind(declaration_id).as_variable_declaration(),
                id.symbol_id.get(),
                reference_symbol(init, self.semantic),
            ) else {
                continue;
            };
            if declaration.kind != VariableDeclarationKind::Const {
                continue;
            }
            if spaces.contains_key(&origin) {
                continue;
            }
            if let Some((origin, reach)) = named.get(&origin).cloned() {
                named.insert(alias, (origin, reach.clone()));
                let exported = matches!(
                    nodes.parent_kind(declaration_id),
                    AstKind::ExportDeclaration(_)
                );
                if exported || scoping.symbol_scope_id(alias) == scoping.root_scope_id() {
                    continue;
                }
                let (name, statement) =
                    self.generated
                        .name(self.code, id.name.as_str(), &reach.0, reach.1.as_deref());
                let after = origin.after;
                if let Some(statement) = statement {
                    self.edits.push((after, after, format!("\n{statement}")));
                }
                for (node, written) in self.references(Some(alias)) {
                    self.replace_binding(id.name.as_str(), id.name.as_str(), &name, node, written);
                }
                let (start, end) = declarator_removal(declaration, declarator.span);
                self.edits
                    .push((start as usize, end as usize, String::new()));
            }
        }
        let added: Vec<Namespace> = namespaces.drain(first_new..).collect();
        for namespace in &added {
            self.read_namespace(namespace);
        }
        namespaces.extend(added);
    }

    /// Moves the `const` a function reads before its declaration, binding
    /// what `css()` or `keyframes()` gives from literals only, above its first
    /// read, as a read after a `const` runs sees it; a read that runs before
    /// it is an error
    pub(super) fn hoist_style_constants(&mut self, program: &Program<'_>, bound: &[Bound<'_, '_>]) {
        let nodes = self.semantic.nodes();
        let apis: FxHashMap<SymbolId, &str> = bound
            .iter()
            .filter_map(|bound| {
                let api = bound.devup.1.as_deref()?;
                HOISTED
                    .contains(&api)
                    .then(|| Some((bound.binding.symbol_id.get()?, api)))?
            })
            .collect();
        let imports: Vec<SymbolId> = bound
            .iter()
            .filter_map(|bound| bound.binding.symbol_id.get())
            .collect();
        for statement in &program.body {
            let Some(Statement::VariableDeclaration(declaration)) = Some(statement) else {
                continue;
            };
            let [declarator] = declaration.declarations.as_slice() else {
                continue;
            };
            let (
                BindingPattern::BindingIdentifier(id),
                Some(Expression::CallExpression(call)),
                VariableDeclarationKind::Const,
            ) = (&declarator.id, &declarator.init, declaration.kind)
            else {
                continue;
            };
            let Expression::Identifier(callee) = &call.callee else {
                continue;
            };
            let Some(api) =
                reference_symbol(callee, self.semantic).and_then(|symbol| apis.get(&symbol))
            else {
                continue;
            };
            let early: Vec<(NodeId, u32)> = self
                .references(id.symbol_id.get())
                .into_iter()
                .map(|(node, _)| (node, nodes.kind(node).span().start))
                .filter(|(_, start)| *start < declaration.span.start)
                .collect();
            if early.is_empty() {
                continue;
            }
            let name = id.name.as_str();
            if let Some((_, start)) = early.iter().find(|(node, _)| !self.is_deferred(*node)) {
                self.errors.push((
                    *start,
                    format!(
                        "`{api}()` cannot use `{name}` at build time: it is read before `const {name} = {api}(…)` runs, so move that declaration above where it is first read"
                    ),
                ));
                continue;
            }
            let literal_only = nodes.iter().all(|node| {
                let AstKind::IdentifierReference(reference) = node.kind() else {
                    return true;
                };
                let inside =
                    reference.span.start >= call.span.start && reference.span.end <= call.span.end;
                !inside
                    || reference_symbol(reference, self.semantic)
                        .is_none_or(|symbol| imports.contains(&symbol))
            });
            if !literal_only {
                continue;
            }
            let first = early
                .iter()
                .map(|(_, start)| *start)
                .min()
                .unwrap_or_default();
            let before = program
                .body
                .iter()
                .find(|statement| statement.span().end > first)
                .map_or(declaration.span.start, |statement| statement.span().start);
            let text = &self.code[declaration.span.start as usize..declaration.span.end as usize];
            self.edits
                .push((before as usize, before as usize, format!("{text}\n")));
            self.edits.push((
                declaration.span.start as usize,
                declaration.span.end as usize,
                String::new(),
            ));
        }
    }
}
