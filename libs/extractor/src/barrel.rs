//! Devup UI reached through something other than a named import of the
//! package: a project module re-exporting it (a barrel), or a namespace import
//! read for its members.
//!
//! Both are rewritten to the named imports of the package itself, so a file
//! compiles as it does importing them from the package, and what the build
//! cannot follow exactly is reported where it is used.

use std::collections::BTreeSet;
use std::rc::Rc;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingIdentifier, BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression,
    ImportDeclarationSpecifier, Statement, StaticMemberExpression, VariableDeclarationKind,
};
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::node::NodeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::component::ExportVariableKind;
use crate::import_alias_visit::Edit;
use crate::module_loader::declared_names;
use crate::util_type::UtilType;
use crate::utils::is_vanilla_extract_file;
use crate::{ModuleResolver, ResolvedModule};

/// How a module gives a name it exports
#[derive(Clone)]
enum Link {
    /// Declared in the module
    Own,
    /// Taken from `source`: `imported` is the name read from it, `None` its
    /// namespace
    From {
        source: String,
        imported: Option<String>,
    },
}

/// What a module exports, read without running it
struct Exports {
    path: String,
    named: FxHashMap<String, Link>,
    /// Modules re-exported whole with `export * from`
    stars: Vec<String>,
    mentions_package: bool,
    /// Nothing in it can lead to the package
    trivial: bool,
}

/// Where reading a name from a module leads
enum Origin {
    /// `imported` of `source`, a module of the package; `None` is its namespace
    Devup {
        source: String,
        imported: Option<String>,
    },
    /// Something else: a name the module declares, or takes from elsewhere
    Other,
    /// A re-export of the package the build cannot follow, and why
    Unfollowable(String),
    /// The module does not export it
    Absent,
}

/// Whether `code` may re-export names from other modules
fn has_reexport_syntax(code: &str) -> bool {
    code.match_indices("export").any(|(index, keyword)| {
        code[index + keyword.len()..]
            .trim_start()
            .starts_with(['{', '*'])
    })
}

fn analyze(module: &ResolvedModule, package: &str) -> Exports {
    let mentions_package = module.code.contains(package);
    let mut exports = Exports {
        path: module.path.clone(),
        named: FxHashMap::default(),
        stars: Vec::new(),
        mentions_package,
        trivial: !mentions_package && !has_reexport_syntax(&module.code),
    };
    if exports.trivial {
        return exports;
    }
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(&module.path).unwrap_or_default();
    let program = Parser::new(&allocator, &module.code, source_type)
        .parse()
        .program;
    let mut imports: FxHashMap<&str, Link> = FxHashMap::default();
    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(import) if !import.import_kind.is_type() => {
                for specifier in import.specifiers.iter().flatten() {
                    let (local, imported) = match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(named) => {
                            (&named.local, Some(named.imported.name().to_string()))
                        }
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                            (&default.local, Some("default".to_string()))
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                            (&namespace.local, None)
                        }
                    };
                    imports.insert(
                        local.name.as_str(),
                        Link::From {
                            source: import.source.value.to_string(),
                            imported,
                        },
                    );
                }
            }
            Statement::ExportFromDeclaration(export) if !export.export_kind.is_type() => {
                for specifier in export
                    .specifiers
                    .iter()
                    .filter(|s| !s.export_kind.is_type())
                {
                    exports.named.insert(
                        specifier.exported.name().to_string(),
                        Link::From {
                            source: export.source.value.to_string(),
                            imported: Some(specifier.local.name().to_string()),
                        },
                    );
                }
            }
            Statement::ExportAllDeclaration(export) if !export.export_kind.is_type() => {
                let source = export.source.value.to_string();
                match &export.exported {
                    Some(name) => {
                        exports.named.insert(
                            name.name().to_string(),
                            Link::From {
                                source,
                                imported: None,
                            },
                        );
                    }
                    None => exports.stars.push(source),
                }
            }
            Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => {
                for specifier in export
                    .specifiers
                    .iter()
                    .filter(|s| !s.export_kind.is_type())
                {
                    let link = imports
                        .get(specifier.local.name().as_str())
                        .cloned()
                        .unwrap_or(Link::Own);
                    exports
                        .named
                        .insert(specifier.exported.name().to_string(), link);
                }
            }
            Statement::VariableDeclaration(declaration) => {
                for (name, link) in alias_links(declaration, &imports) {
                    imports.insert(name, link);
                }
            }
            Statement::ExportDeclaration(export) => {
                for name in declared_names(&export.declaration) {
                    exports.named.insert(name, Link::Own);
                }
                if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                    for (name, link) in alias_links(declaration, &imports) {
                        exports.named.insert(name.to_string(), link.clone());
                        imports.insert(name, link);
                    }
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let link = match &export.declaration {
                    ExportDefaultDeclarationKind::Identifier(name) => {
                        imports.get(name.name.as_str()).cloned()
                    }
                    _ => None,
                };
                exports
                    .named
                    .insert("default".to_string(), link.unwrap_or(Link::Own));
            }
            _ => {}
        }
    }
    exports
}

/// Where `Namespace.member` leads, for a namespace a module imports
fn member_link(
    member: &StaticMemberExpression<'_>,
    imports: &FxHashMap<&str, Link>,
) -> Option<Link> {
    if let Expression::Identifier(object) = &member.object
        && let Some(Link::From {
            source,
            imported: None,
        }) = imports.get(object.name.as_str())
    {
        Some(Link::From {
            source: source.clone(),
            imported: Some(member.property.name.to_string()),
        })
    } else {
        None
    }
}

/// The bindings a `const` declaration makes of what a module imports
/// (`const B = Box`, `const C = Devup.css`), with where each leads
fn alias_links<'a>(
    declaration: &oxc_ast::ast::VariableDeclaration<'a>,
    imports: &FxHashMap<&str, Link>,
) -> Vec<(&'a str, Link)> {
    if declaration.kind != VariableDeclarationKind::Const {
        return Vec::new();
    }
    declaration
        .declarations
        .iter()
        .filter_map(|declarator| {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                return None;
            };
            let link = match declarator.init.as_ref()? {
                Expression::Identifier(init) => imports.get(init.name.as_str())?.clone(),
                Expression::StaticMemberExpression(member) => member_link(member, imports)?,
                _ => return None,
            };
            Some((id.name.as_str(), link))
        })
        .collect()
}

/// Follows re-exports through the modules a resolver reads
struct Walker<'r, 'p> {
    resolver: &'r ModuleResolver,
    package: &'p str,
    modules: FxHashMap<String, Rc<Exports>>,
    dependencies: BTreeSet<String>,
}

impl Walker<'_, '_> {
    fn is_package(&self, source: &str) -> bool {
        source == self.package
            || source
                .strip_prefix(self.package)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    fn module(&mut self, specifier: &str, importer: &str) -> Option<Rc<Exports>> {
        let resolved = (self.resolver)(specifier, importer)?;
        self.dependencies.insert(resolved.path.clone());
        Some(
            self.modules
                .entry(resolved.path.clone())
                .or_insert_with(|| Rc::new(analyze(&resolved, self.package)))
                .clone(),
        )
    }

    /// Where `imported` of `source`, imported by `importer`, leads
    fn import_origin(&mut self, source: &str, importer: &str, imported: &str) -> Origin {
        match self.module(source, importer) {
            None => Origin::Other,
            Some(module) => match self.origin(&module, imported, &mut Vec::new()) {
                Origin::Absent => Origin::Other,
                origin => origin,
            },
        }
    }

    fn origin(
        &mut self,
        module: &Exports,
        name: &str,
        reading: &mut Vec<(String, String)>,
    ) -> Origin {
        let key = (module.path.clone(), name.to_string());
        if module.trivial || reading.contains(&key) {
            return Origin::Absent;
        }
        reading.push(key);
        let origin = match module.named.get(name) {
            Some(Link::Own) => Origin::Other,
            Some(Link::From { source, imported }) => {
                self.follow(module, source, imported.as_deref(), reading)
            }
            None if name == "default" => Origin::Absent,
            None => self.through_stars(module, name, reading),
        };
        reading.pop();
        origin
    }

    fn follow(
        &mut self,
        module: &Exports,
        source: &str,
        imported: Option<&str>,
        reading: &mut Vec<(String, String)>,
    ) -> Origin {
        if self.is_package(source) {
            return Origin::Devup {
                source: source.to_string(),
                imported: imported.map(str::to_string),
            };
        }
        let Some(next) = self.module(source, &module.path) else {
            return if module.mentions_package {
                Origin::Unfollowable(format!("`{source}` cannot be read"))
            } else {
                Origin::Other
            };
        };
        match imported {
            Some(name) => match self.origin(&next, name, reading) {
                Origin::Absent => Origin::Other,
                origin => origin,
            },
            None if self.touches_package(&next, &mut FxHashSet::default()) => {
                Origin::Unfollowable(format!(
                    "`{source}` is a namespace of modules re-exporting `{}`",
                    self.package
                ))
            }
            None => Origin::Other,
        }
    }

    fn through_stars(
        &mut self,
        module: &Exports,
        name: &str,
        reading: &mut Vec<(String, String)>,
    ) -> Origin {
        let (mut strong, mut weak, mut unfollowable) = (None, None, None);
        for star in &module.stars {
            if self.is_package(star) {
                weak = Some(Origin::Devup {
                    source: star.clone(),
                    imported: Some(name.to_string()),
                });
            } else if let Some(next) = self.module(star, &module.path) {
                match self.origin(&next, name, reading) {
                    Origin::Absent => {}
                    found => strong = strong.or(Some(found)),
                }
            } else if module.mentions_package {
                unfollowable = Some(Origin::Unfollowable(format!("`{star}` cannot be read")));
            }
        }
        strong.or(weak).or(unfollowable).unwrap_or(Origin::Absent)
    }

    /// Whether `module` re-exports something from the package, however far
    fn touches_package(&mut self, module: &Exports, seen: &mut FxHashSet<String>) -> bool {
        if !seen.insert(module.path.clone()) {
            return false;
        }
        let sources = module
            .named
            .values()
            .filter_map(|link| match link {
                Link::From { source, .. } => Some(source),
                Link::Own => None,
            })
            .chain(&module.stars);
        sources.into_iter().any(|source| {
            self.is_package(source)
                || self
                    .module(source, &module.path)
                    .is_some_and(|next| self.touches_package(&next, seen))
        })
    }
}

/// What a file's imports of Devup UI became
pub(crate) enum Barreled {
    Unchanged,
    Rewritten(Rewritten),
    /// What the build cannot follow, as `(offset, message)` in the source
    Failed(Vec<(u32, String)>),
}

pub(crate) struct Rewritten {
    pub code: String,
    /// The replacements made in order, so a position in `code` maps back to
    /// the source
    pub edits: Vec<Edit>,
    /// The modules read to follow re-exports
    pub dependencies: BTreeSet<String>,
}

/// A namespace or default import whose members are read as values
struct NsDecl<'p, 'a> {
    binding: &'p BindingIdentifier<'a>,
    /// Where its declaration ends, after which the named imports it needs go
    after: usize,
    /// The barrel module it is a namespace of; `None` for the package
    module: Option<Rc<Exports>>,
    source: String,
}

/// A binding of a namespace, known once the program is analysed
struct Namespace {
    local: String,
    symbol: Option<SymbolId>,
    /// Where its declaration ends, after which the named imports it needs go
    after: usize,
    /// The barrel module it is a namespace of; `None` for the package
    module: Option<Rc<Exports>>,
    source: String,
}

/// The names the rewritten file gives what it takes from namespaces
#[derive(Default)]
struct Generated {
    names: FxHashMap<(String, Option<String>), String>,
    taken: FxHashSet<String>,
}

impl Generated {
    /// The local name of `imported` of `source`, with the statement importing
    /// it when the name is new
    fn name(
        &mut self,
        code: &str,
        namespace: &str,
        source: &str,
        imported: Option<&str>,
    ) -> (String, Option<String>) {
        let key = (source.to_string(), imported.map(str::to_string));
        if let Some(name) = self.names.get(&key) {
            return (name.clone(), None);
        }
        let mut name = format!("{namespace}${}", imported.unwrap_or("namespace"));
        while code.contains(&name) || self.taken.contains(&name) {
            name.push('$');
        }
        let quoted = serde_json::to_string(source).unwrap_or_default();
        let statement = match imported {
            Some(imported) => format!("import {{ {imported} as {name} }} from {quoted};"),
            None => format!("import * as {name} from {quoted};"),
        };
        self.taken.insert(name.clone());
        self.names.insert(key, name.clone());
        (name, Some(statement))
    }
}

mod aliases;
use aliases::{Reach, declarator_removal, export_edits, is_assigned_from};

fn unreadable(local: &str, code: &str) -> String {
    format!(
        "`{local}` cannot use `{code}` at build time: read its members by name, as `{local}.css`, `{local}['css']` or `const {{ css }} = {local}`, or import them by name"
    )
}

fn unfollowable(code: &str, specifier: &str, reason: &str) -> String {
    format!(
        "`{code}` cannot use `{specifier}` at build time: the build cannot follow its re-export of Devup UI ({reason}); import it from the package instead"
    )
}

/// The member a computed key names, when it is a literal
fn literal_key(key: &Expression<'_>) -> Option<String> {
    match key {
        Expression::StringLiteral(text) => Some(text.value.to_string()),
        Expression::TemplateLiteral(template) => match template.quasis.as_slice() {
            [only] => only.value.cooked.as_ref().map(ToString::to_string),
            _ => None,
        },
        _ => None,
    }
}

/// The import of `walker`'s modules that reach the package and are namespaces
fn namespace_of(walker: &mut Walker<'_, '_>, source: &str, importer: &str) -> Option<Rc<Exports>> {
    let module = walker.module(source, importer)?;
    walker
        .touches_package(&module, &mut FxHashSet::default())
        .then_some(module)
}

/// Whether the build compiles the package export `name` away; the others are
/// values a namespace holds at runtime as they are
fn compiled_export(name: &str) -> bool {
    name.parse::<ExportVariableKind>().is_ok()
        || UtilType::from_str_opt(name).is_some()
        || name == "styled"
}

/// The specifiers of an import a statement keeps
#[derive(Default)]
struct Kept<'c> {
    default: Option<&'c str>,
    namespace: Option<&'c str>,
    named: Vec<&'c str>,
}

/// An import that no rewriting can follow exactly: the binding, where it
/// comes from, and why
struct Opaque<'p, 'a> {
    binding: &'p BindingIdentifier<'a>,
    source: String,
    reason: String,
}

/// A named or default import that is Devup UI: the binding, what it imports
/// from where, and what that is in the package
struct Bound<'p, 'a> {
    binding: &'p BindingIdentifier<'a>,
    /// Where the import declaration ends
    after: usize,
    source: String,
    imported: String,
    devup: (String, Option<String>),
}

#[derive(Default)]
struct Found<'p, 'a> {
    bound: Vec<Bound<'p, 'a>>,
    namespaces: Vec<NsDecl<'p, 'a>>,
    opaque: Vec<Opaque<'p, 'a>>,
}

/// The statements importing what an import takes from the package through a
/// project module, with the rest of it, or `None` when none goes there
fn redirect_import<'p, 'a>(
    walker: &mut Walker<'_, '_>,
    code: &str,
    filename: &str,
    import: &'p oxc_ast::ast::ImportDeclaration<'a>,
    found: &mut Found<'p, 'a>,
) -> Option<String> {
    let source = import.source.value.as_str();
    let after = import.span.end as usize;
    let mut redirected: Vec<(Option<String>, &str, String)> = Vec::new();
    let mut kept = Kept::default();
    for specifier in import.specifiers.iter().flatten() {
        let (imported, local, is_default) = match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(named) if !named.import_kind.is_type() => {
                (named.imported.name().to_string(), &named.local, false)
            }
            ImportDeclarationSpecifier::ImportSpecifier(named) => {
                kept.named
                    .push(&code[named.span.start as usize..named.span.end as usize]);
                continue;
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                ("default".to_string(), &default.local, true)
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                kept.namespace = Some(namespace.local.name.as_str());
                if let Some(module) = namespace_of(walker, source, filename) {
                    found.namespaces.push(NsDecl {
                        binding: &namespace.local,
                        after,
                        module: Some(module),
                        source: source.to_string(),
                    });
                }
                continue;
            }
        };
        match walker.import_origin(source, filename, &imported) {
            Origin::Devup {
                source: from,
                imported: target,
            } => {
                found.bound.push(Bound {
                    binding: local,
                    after,
                    source: source.to_string(),
                    imported: imported.clone(),
                    devup: (from.clone(), target.clone()),
                });
                redirected.push((target, local.name.as_str(), from));
            }
            origin => {
                if let Origin::Unfollowable(reason) = origin {
                    found.opaque.push(Opaque {
                        binding: local,
                        source: source.to_string(),
                        reason,
                    });
                }
                let span = specifier.span();
                if is_default {
                    kept.default = Some(local.name.as_str());
                } else {
                    kept.named
                        .push(&code[span.start as usize..span.end as usize]);
                }
            }
        }
    }
    if redirected.is_empty() {
        return None;
    }
    let mut statements: Vec<String> = Vec::new();
    let mut named: Vec<(String, Vec<String>)> = Vec::new();
    for (imported, local, from) in redirected {
        let quoted = serde_json::to_string(&from).unwrap_or_default();
        match imported {
            None => statements.push(format!("import * as {local} from {quoted};")),
            Some(imported) => {
                let part = if imported == local {
                    imported
                } else {
                    format!("{imported} as {local}")
                };
                match named.iter_mut().find(|(source, _)| *source == quoted) {
                    Some((_, parts)) => parts.push(part),
                    None => named.push((quoted, vec![part])),
                }
            }
        }
    }
    statements.extend(
        named
            .into_iter()
            .map(|(quoted, parts)| format!("import {{ {} }} from {quoted};", parts.join(", "))),
    );
    let mut parts: Vec<String> = Vec::new();
    parts.extend(kept.default.map(str::to_string));
    parts.extend(kept.namespace.map(|name| format!("* as {name}")));
    if !kept.named.is_empty() {
        parts.push(format!("{{ {} }}", kept.named.join(", ")));
    }
    if !parts.is_empty() {
        let quoted = serde_json::to_string(source).unwrap_or_default();
        statements.push(format!("import {} from {quoted};", parts.join(", ")));
    }
    Some(statements.join("\n"))
}

/// Rewrites the members read from namespaces, collecting what it cannot
struct Rewriter<'s, 'w, 'r, 'p> {
    code: &'s str,
    semantic: &'s Semantic<'s>,
    walker: &'w mut Walker<'r, 'p>,
    generated: Generated,
    edits: Vec<(usize, usize, String)>,
    errors: Vec<(u32, String)>,
}

impl Rewriter<'_, '_, '_, '_> {
    /// Where reading `name` from `namespace` leads
    fn member_origin(&mut self, namespace: &Namespace, name: &str) -> Origin {
        match &namespace.module {
            None if !compiled_export(name) => Origin::Other,
            None => Origin::Devup {
                source: namespace.source.clone(),
                imported: Some(name.to_string()),
            },
            Some(module) => match self.walker.origin(module, name, &mut Vec::new()) {
                Origin::Absent => Origin::Other,
                origin => origin,
            },
        }
    }

    /// The value references of `binding`, as `(node, is written)`
    fn references(&self, symbol: Option<SymbolId>) -> Vec<(NodeId, bool)> {
        let scoping = self.semantic.scoping();
        symbol.map_or_else(Vec::new, |symbol| {
            scoping
                .get_resolved_reference_ids(symbol)
                .iter()
                .map(|id| scoping.get_reference(*id))
                .filter(|reference| reference.is_value())
                .map(|reference| {
                    let flags = reference.flags();
                    (
                        reference.node_id(),
                        flags.is_write() || flags.is_member_write_target(),
                    )
                })
                .collect()
        })
    }

    fn whole_error(&mut self, local: &str, start: u32, end: u32) {
        let text = &self.code[start as usize..end as usize];
        let code = match text.char_indices().nth(60) {
            Some((index, _)) => format!("{}…", &text[..index]),
            None => text.to_string(),
        };
        self.errors.push((start, unreadable(local, &code)));
    }

    fn read_namespace(&mut self, namespace: &Namespace) {
        let mut appended: Vec<String> = Vec::new();
        for (node, written) in self.references(namespace.symbol) {
            self.read_reference(namespace, &mut appended, node, written);
        }
        if !appended.is_empty() {
            self.edits.push((
                namespace.after,
                namespace.after,
                format!("\n{}", appended.join("\n")),
            ));
        }
    }

    fn read_reference(
        &mut self,
        namespace: &Namespace,
        appended: &mut Vec<String>,
        node: NodeId,
        written: bool,
    ) {
        let nodes = self.semantic.nodes();
        let span = nodes.kind(node).span();
        let parent = nodes.parent_kind(node);
        let local = namespace.local.as_str();
        let member = match parent {
            AstKind::StaticMemberExpression(member) => {
                Some((member.span, member.property.name.to_string()))
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                literal_key(&member.expression).map(|name| (member.span, name))
            }
            AstKind::JSXMemberExpression(member) => {
                Some((member.span, member.property.name.to_string()))
            }
            _ => None,
        };
        match (member, parent) {
            _ if written => {
                let span = parent.span();
                self.whole_error(local, span.start, span.end);
            }
            (Some((member_span, name)), _) => {
                let origin = self.member_origin(namespace, &name);
                self.replace(namespace, appended, member_span, &name, origin);
            }
            (None, AstKind::VariableDeclarator(declarator))
                if matches!(declarator.id, BindingPattern::BindingIdentifier(_)) => {}
            (None, AstKind::VariableDeclarator(declarator))
                if declarator
                    .init
                    .as_ref()
                    .is_some_and(|init| init.span() == span) =>
            {
                self.destructure(namespace, appended, nodes.parent_id(node), declarator);
            }
            (None, AstKind::ComputedMemberExpression(member)) if member.object.span() == span => {
                self.whole_error(local, member.span.start, member.span.end);
            }
            (None, _) => {}
        }
    }

    /// Replaces `span`, which reads `name` of the namespace, by what the
    /// package gives it
    fn replace(
        &mut self,
        namespace: &Namespace,
        appended: &mut Vec<String>,
        span: oxc_span::Span,
        name: &str,
        origin: Origin,
    ) {
        match origin {
            Origin::Devup { source, imported } => {
                let (local, statement) = self.generated.name(
                    self.code,
                    namespace.local.as_str(),
                    &source,
                    imported.as_deref(),
                );
                appended.extend(statement);
                self.edits
                    .push((span.start as usize, span.end as usize, local));
            }
            Origin::Unfollowable(reason) => {
                let code = format!("{}.{name}", namespace.local);
                self.errors
                    .push((span.start, unfollowable(&code, &namespace.source, &reason)));
            }
            Origin::Other | Origin::Absent => {}
        }
    }

    /// `const { css, Box: B } = Namespace`: the declaration goes, and what it
    /// bound is read from the package where it is used
    fn destructure(
        &mut self,
        namespace: &Namespace,
        appended: &mut Vec<String>,
        declarator_id: NodeId,
        declarator: &oxc_ast::ast::VariableDeclarator<'_>,
    ) {
        let local = namespace.local.as_str();
        let whole = |rewriter: &mut Self| {
            rewriter.whole_error(local, declarator.span.start, declarator.span.end);
        };
        let BindingPattern::ObjectPattern(pattern) = &declarator.id else {
            return whole(self);
        };
        if pattern.rest.is_some() {
            return whole(self);
        }
        let mut members = Vec::new();
        for property in &pattern.properties {
            match (&property.value, property.key.static_name()) {
                (BindingPattern::BindingIdentifier(binding), Some(name)) if !property.computed => {
                    members.push((name.to_string(), binding));
                }
                _ => return whole(self),
            }
        }
        let mut devup = Vec::new();
        let mut unfollowed = None;
        for (name, binding) in &members {
            match self.member_origin(namespace, name) {
                Origin::Devup { source, imported } => devup.push((name, binding, source, imported)),
                Origin::Unfollowable(reason) => unfollowed = unfollowed.or(Some(reason)),
                Origin::Other | Origin::Absent => {}
            }
        }
        if let Some(reason) = unfollowed {
            let code = &self.code[declarator.span.start as usize..declarator.span.end as usize];
            self.errors.push((
                declarator.span.start,
                unfollowable(code, &namespace.source, &reason),
            ));
            return;
        }
        if devup.is_empty() {
            return;
        }
        let nodes = self.semantic.nodes();
        let declaration_id = nodes.parent_id(declarator_id);
        let declaration = match nodes.kind(declaration_id) {
            AstKind::VariableDeclaration(declaration)
                if devup.len() == members.len()
                    && !matches!(
                        nodes.parent_kind(declaration_id),
                        AstKind::ExportDeclaration(_)
                    ) =>
            {
                declaration
            }
            _ => return whole(self),
        };
        let (start, end) = declarator_removal(declaration, declarator.span);
        self.edits
            .push((start as usize, end as usize, String::new()));
        for (name, binding, source, imported) in devup {
            let (generated, statement) =
                self.generated
                    .name(self.code, local, &source, imported.as_deref());
            appended.extend(statement);
            for (node, written) in self.references(binding.symbol_id.get()) {
                self.replace_binding(local, name, &generated, node, written);
            }
        }
    }

    /// A read of what `const { name } = Namespace` bound, as `generated`
    fn replace_binding(
        &mut self,
        namespace: &str,
        name: &str,
        generated: &str,
        node: NodeId,
        written: bool,
    ) {
        let nodes = self.semantic.nodes();
        let span = nodes.kind(node).span();
        let text = match nodes.parent_kind(node) {
            _ if written => {
                return self.whole_error(namespace, span.start, span.end);
            }
            AstKind::ObjectProperty(property) if property.shorthand => {
                format!(
                    "{}: {generated}",
                    &self.code[span.start as usize..span.end as usize]
                )
            }
            AstKind::ExportSpecifier(specifier) if specifier.span == span => {
                format!("{generated} as {name}")
            }
            _ => generated.to_string(),
        };
        self.edits
            .push((span.start as usize, span.end as usize, text));
    }
}

/// Rewrites the imports of `code` that reach Devup UI through a project module
/// or a namespace
pub(crate) fn rewrite(
    code: &str,
    filename: &str,
    package: &str,
    resolver: Option<&ModuleResolver>,
) -> Barreled {
    if is_vanilla_extract_file(filename) || (resolver.is_none() && !code.contains(package)) {
        return Barreled::Unchanged;
    }
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let ParserReturn {
        program,
        fatal_error,
        ..
    } = Parser::new(&allocator, code, source_type).parse();
    if fatal_error {
        return Barreled::Unchanged;
    }
    let no_modules = |_: &str, _: &str| -> Option<ResolvedModule> { None };
    let mut walker = Walker {
        resolver: resolver.unwrap_or(&no_modules),
        package,
        modules: FxHashMap::default(),
        dependencies: BTreeSet::new(),
    };
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut found = Found::default();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        if import.import_kind.is_type() || import.with_clause.is_some() {
            continue;
        }
        if walker.is_package(import.source.value.as_str()) {
            for specifier in import.specifiers.iter().flatten() {
                let binding = match specifier {
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => &default.local,
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                        &namespace.local
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(named) => {
                        if !named.import_kind.is_type() {
                            let imported = named.imported.name().to_string();
                            found.bound.push(Bound {
                                binding: &named.local,
                                after: import.span.end as usize,
                                source: import.source.value.to_string(),
                                devup: (import.source.value.to_string(), Some(imported.clone())),
                                imported,
                            });
                        }
                        continue;
                    }
                };
                found.namespaces.push(NsDecl {
                    binding,
                    after: import.span.end as usize,
                    module: None,
                    source: import.source.value.to_string(),
                });
            }
        } else if let Some(statements) =
            redirect_import(&mut walker, code, filename, import, &mut found)
        {
            edits.push((
                import.span.start as usize,
                import.span.end as usize,
                statements,
            ));
        }
    }
    let locals: FxHashMap<String, Reach> = found
        .bound
        .iter()
        .filter(|bound| bound.devup.1.as_deref().is_some_and(compiled_export))
        .map(|bound| {
            (
                bound.binding.name.to_string(),
                (bound.source.clone(), Some(bound.imported.clone())),
            )
        })
        .chain(found.namespaces.iter().map(|declared| {
            (
                declared.binding.name.to_string(),
                (declared.source.clone(), None),
            )
        }))
        .collect();
    edits.extend(export_edits(&program, code, locals));
    let wants_semantic = !found.namespaces.is_empty()
        || !found.opaque.is_empty()
        || found
            .bound
            .iter()
            .any(|bound| is_assigned_from(code, bound.binding.name.as_str()))
        || (!found.bound.is_empty() && (code.contains("= keyframes(") || code.contains("= css(")));
    let (mut edits, errors) = if wants_semantic {
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(&program)
            .semantic;
        let mut rewriter = Rewriter {
            code,
            semantic: &semantic,
            walker: &mut walker,
            generated: Generated::default(),
            edits,
            errors: Vec::new(),
        };
        for opaque in &found.opaque {
            let local = opaque.binding.name.as_str();
            for (node, _) in rewriter.references(opaque.binding.symbol_id.get()) {
                let span = semantic.nodes().kind(node).span();
                rewriter.errors.push((
                    span.start,
                    unfollowable(local, &opaque.source, &opaque.reason),
                ));
            }
        }
        let mut spaces: Vec<Namespace> = found
            .namespaces
            .iter()
            .map(|declared| Namespace {
                local: declared.binding.name.to_string(),
                symbol: declared.binding.symbol_id.get(),
                after: declared.after,
                module: declared.module.clone(),
                source: declared.source.clone(),
            })
            .collect();
        for namespace in &spaces {
            rewriter.read_namespace(namespace);
        }
        rewriter.follow_aliases(&found.bound, &mut spaces);
        rewriter.hoist_style_constants(&program, &found.bound);
        (rewriter.edits, rewriter.errors)
    } else {
        (edits, Vec::new())
    };
    if !errors.is_empty() {
        return Barreled::Failed(errors);
    }
    if edits.is_empty() {
        return Barreled::Unchanged;
    }
    edits.sort_by_key(|(start, end, _)| (*start, *end));
    let recorded = edits
        .iter()
        .map(|(start, end, text)| (*start, *end, text.len()))
        .collect();
    let mut result = code.to_string();
    for (start, end, text) in edits.into_iter().rev() {
        result.replace_range(start..end, &text);
    }
    Barreled::Rewritten(Rewritten {
        code: result,
        edits: recorded,
        dependencies: walker.dependencies,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests;
