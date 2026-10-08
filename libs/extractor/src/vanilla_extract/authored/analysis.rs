use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, ModuleExportName, Program, Statement,
};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub(super) struct Unit {
    pub span: Span,
    pub text: String,
    pub symbols: Vec<SymbolId>,
}

pub(super) struct Export {
    pub target: Option<SymbolId>,
    pub span: Span,
    pub text: String,
}

pub(super) struct Index<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub code: &'s str,
    pub units: FxHashMap<SymbolId, Vec<Unit>>,
    pub exports: FxHashMap<String, Export>,
    pub stars: Vec<Span>,
}

impl<'s, 'a> Index<'s, 'a> {
    pub fn new(program: &Program<'a>, semantic: &'s Semantic<'a>, code: &'s str) -> Self {
        let mut index = Self {
            semantic,
            code,
            units: FxHashMap::default(),
            exports: FxHashMap::default(),
            stars: Vec::new(),
        };
        for statement in &program.body {
            match statement {
                Statement::ExportDeclaration(export) => {
                    index.declaration(&export.declaration, true);
                }
                Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => {
                    for specifier in &export.specifiers {
                        if specifier.export_kind.is_type() {
                            continue;
                        }
                        let target = match &specifier.local {
                            ModuleExportName::IdentifierReference(identifier) => identifier
                                .reference_id
                                .get()
                                .and_then(|id| semantic.scoping().get_reference(id).symbol_id()),
                            _ => None,
                        };
                        index.exports.insert(
                            specifier.exported.name().to_string(),
                            Export {
                                target,
                                span: specifier.span,
                                text: format!("export {{ {} }};", specifier.span.source_text(code)),
                            },
                        );
                    }
                }
                Statement::ExportDefaultDeclaration(export) => {
                    let declaration = &export.declaration;
                    let target = match declaration {
                        ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                            function.id.as_ref().and_then(|id| id.symbol_id.get())
                        }
                        ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                            class.id.as_ref().and_then(|id| id.symbol_id.get())
                        }
                        ExportDefaultDeclarationKind::Identifier(identifier) => identifier
                            .reference_id
                            .get()
                            .and_then(|id| semantic.scoping().get_reference(id).symbol_id()),
                        _ => None,
                    };
                    let owns_binding = matches!(
                        declaration,
                        ExportDefaultDeclarationKind::FunctionDeclaration(_)
                            | ExportDefaultDeclarationKind::ClassDeclaration(_)
                    );
                    if owns_binding && let Some(symbol) = target {
                        index.units.entry(symbol).or_default().push(Unit {
                            span: export.span,
                            text: export.span.source_text(code).to_string(),
                            symbols: vec![symbol],
                        });
                    }
                    index.exports.insert(
                        "default".into(),
                        Export {
                            target,
                            span: export.span,
                            text: if owns_binding && target.is_some() {
                                String::new()
                            } else {
                                export.span.source_text(code).to_string()
                            },
                        },
                    );
                }
                Statement::ExportFromDeclaration(export) if !export.export_kind.is_type() => {
                    for specifier in &export.specifiers {
                        if !specifier.export_kind.is_type() {
                            index.exports.insert(
                                specifier.exported.name().to_string(),
                                Export {
                                    target: None,
                                    span: specifier.span,
                                    text: format!(
                                        "export {{ {} }} from {};",
                                        specifier.span.source_text(code),
                                        export.source.span.source_text(code)
                                    ),
                                },
                            );
                        }
                    }
                }
                Statement::ExportAllDeclaration(export) if !export.export_kind.is_type() => {
                    if let Some(name) = &export.exported {
                        index.exports.insert(
                            name.name().to_string(),
                            Export {
                                target: None,
                                span: export.span,
                                text: export.span.source_text(code).to_string(),
                            },
                        );
                    } else {
                        index.stars.push(export.span);
                    }
                }
                statement => {
                    if let Some(declaration) = statement.as_declaration() {
                        index.declaration(declaration, false);
                    }
                }
            }
        }
        index
    }

    fn declaration(&mut self, declaration: &Declaration<'a>, exported: bool) {
        if let Declaration::VariableDeclaration(declaration) = declaration {
            if declaration.declare {
                return;
            }
            for declarator in &declaration.declarations {
                let symbols: Vec<_> = declarator
                    .id
                    .get_binding_identifiers()
                    .iter()
                    .filter_map(|id| id.symbol_id.get())
                    .collect();
                let unit = Unit {
                    span: declarator.span,
                    text: format!(
                        "{}{} {};",
                        if exported { "export " } else { "" },
                        declaration.kind.as_str(),
                        declarator.span.source_text(self.code)
                    ),
                    symbols,
                };
                self.insert(unit, exported);
            }
        } else if let Some(id) = declaration.id()
            && let Some(symbol) = id.symbol_id.get()
        {
            let unit = Unit {
                span: declaration.span(),
                text: format!(
                    "{}{}",
                    if exported { "export " } else { "" },
                    declaration.span().source_text(self.code)
                ),
                symbols: vec![symbol],
            };
            self.insert(unit, exported);
        }
    }

    fn insert(&mut self, unit: Unit, exported: bool) {
        for symbol in &unit.symbols {
            self.units.entry(*symbol).or_default().push(unit.clone());
            if exported {
                self.exports.insert(
                    self.semantic.scoping().symbol_name(*symbol).to_string(),
                    Export {
                        target: Some(*symbol),
                        span: unit.span,
                        text: String::new(),
                    },
                );
            }
        }
    }
}
