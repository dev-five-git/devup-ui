use oxc_allocator::UnstableAddress;
use oxc_ast::ast::{
    Declaration, ExportAllDeclaration, ExportDefaultDeclaration, ExportFromDeclaration,
    ExportNamedDeclaration, ImportDeclarationSpecifier, Program, Statement,
};

use super::{
    callable::Proof,
    origins::{export_name, local_name, origin},
    scopes::pattern_names,
    text::Text,
    traversal::Collector,
    wire::{Origin, insert_request},
};

impl Collector {
    pub fn prepare(&mut self, program: &Program<'_>) {
        if let Some(callables) = &mut self.callables {
            callables.prepare(program);
        }
        let mut import_only = (self.facts.observation.is_some()
            && program.directives.is_empty()
            && !program.body.is_empty())
        .then(Vec::new);
        for statement in &program.body {
            let (source, attributes) = match statement {
                Statement::ImportDeclaration(node) => {
                    (Some(&node.source), node.with_clause.as_deref())
                }
                Statement::ExportFromDeclaration(node) => {
                    (Some(&node.source), node.with_clause.as_deref())
                }
                Statement::ExportAllDeclaration(node) => {
                    (Some(&node.source), node.with_clause.as_deref())
                }
                _ => (None, None),
            };
            let attributed =
                attributes.is_some_and(|attributes| !attributes.with_entries.is_empty());
            if let (Some(proof), Some(source)) = (&mut self.facts.observation, source)
                && attributed
            {
                insert_request(&mut proof.unsupported_requests, Text::literal(source));
            }
            match statement {
                Statement::ImportDeclaration(node) => {
                    if node
                        .specifiers
                        .as_ref()
                        .is_none_or(|specifiers| specifiers.is_empty())
                        && !attributed
                    {
                        if let Some(requests) = &mut import_only {
                            insert_request(requests, Text::literal(&node.source));
                        }
                    } else {
                        import_only = None;
                    }
                    for specifier in node.specifiers.iter().flatten() {
                        let (name, ids) = match specifier {
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                                (specifier.local.name.as_str(), Vec::new())
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                                (specifier.local.name.as_str(), vec![Text::from("default")])
                            }
                            ImportDeclarationSpecifier::ImportSpecifier(specifier) => (
                                specifier.local.name.as_str(),
                                vec![export_name(&specifier.imported)],
                            ),
                        };
                        self.scopes.entries[0].bindings.insert(
                            name.to_owned(),
                            Some(Origin {
                                request: Text::literal(&node.source),
                                ids,
                                require: false,
                            }),
                        );
                    }
                }
                _ => import_only = None,
            }
        }
        if let Some(proof) = &mut self.facts.observation {
            proof.import_only_requests = import_only;
        }
    }

    pub fn named_export(&mut self, node: &ExportNamedDeclaration<'_>) {
        let scope = self.scopes.at(node.unstable_address());
        for specifier in &node.specifiers {
            let name = export_name(&specifier.exported);
            let value =
                local_name(&specifier.local).and_then(|name| self.scopes.binding(scope, name));
            if name.is_default()
                && let Some(callables) = &mut self.callables
            {
                callables.default =
                    local_name(&specifier.local).map(|name| Proof::Alias(name.to_owned()));
            }
            self.facts.export(name, value);
        }
    }

    pub fn forward_export(&mut self, node: &ExportFromDeclaration<'_>) {
        let scope = self.scopes.at(node.unstable_address());
        for specifier in &node.specifiers {
            let name = export_name(&specifier.exported);
            let request = Text::literal(&node.source);
            if name.is_default()
                && let Some(callables) = &mut self.callables
            {
                callables.default = if request.is_empty() {
                    local_name(&specifier.local).map(|name| Proof::Alias(name.to_owned()))
                } else {
                    None
                };
            }
            let value = if request.is_empty() {
                local_name(&specifier.local).and_then(|name| self.scopes.binding(scope, name))
            } else {
                Some(Origin {
                    request,
                    ids: vec![export_name(&specifier.local)],
                    require: false,
                })
            };
            self.facts.export(name, value);
        }
    }

    pub fn star_export(&mut self, node: &ExportAllDeclaration<'_>) {
        let request = Text::literal(&node.source);
        match node
            .exported
            .as_ref()
            .map(export_name)
            .filter(|name| !name.is_empty())
        {
            Some(name) => self.facts.export(
                name,
                Some(Origin {
                    request,
                    ids: Vec::new(),
                    require: false,
                }),
            ),
            None => self.facts.stars.push(request),
        }
    }

    pub fn declared_export(&mut self, declaration: &Declaration<'_>) {
        let names = match declaration {
            Declaration::VariableDeclaration(node) => node
                .declarations
                .iter()
                .flat_map(|declaration| pattern_names(&declaration.id))
                .collect(),
            Declaration::FunctionDeclaration(node) => vec![
                node.id
                    .as_ref()
                    .map_or("", |id| id.name.as_str())
                    .to_owned(),
            ],
            Declaration::ClassDeclaration(node) => vec![
                node.id
                    .as_ref()
                    .map_or("", |id| id.name.as_str())
                    .to_owned(),
            ],
            _ => Vec::new(),
        };
        for name in names {
            self.facts.export(Text::from(name.as_str()), None);
        }
    }

    pub fn default_export(&mut self, node: &ExportDefaultDeclaration<'_>) {
        let scope = self.scopes.at(node.unstable_address());
        let value = node
            .declaration
            .as_expression()
            .and_then(|node| origin(node, &self.scopes, scope));
        self.facts.export(Text::from("default"), value);
        if let Some(callables) = &mut self.callables {
            callables.default = Some(Proof::default_declaration(&node.declaration));
        }
    }
}
