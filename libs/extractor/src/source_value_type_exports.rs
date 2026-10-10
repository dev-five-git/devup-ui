use oxc_ast::ast::{
    BindingIdentifier, Declaration, ExportDefaultDeclarationKind, Ident, Program, Statement,
};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;

use super::{
    model::{Model, Node},
    syntax,
};

pub(super) fn collect(program: &Program<'_>, scoping: &Scoping, model: &mut Model) {
    for statement in &program.body {
        match statement {
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => {
                    for declaration in &declaration.declarations {
                        for identifier in declaration.id.get_binding_identifiers() {
                            bind(identifier, model);
                        }
                    }
                }
                Declaration::FunctionDeclaration(function) => {
                    if let Some(identifier) = &function.id {
                        bind(identifier, model);
                    }
                }
                Declaration::TSTypeAliasDeclaration(alias) => bind(&alias.id, model),
                Declaration::TSInterfaceDeclaration(interface) => bind(&interface.id, model),
                _ => {}
            },
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    let node = scoping
                        .get_root_binding(Ident::from(specifier.local.name().as_str()))
                        .map_or(Node::UNKNOWN, Node::Symbol);
                    model
                        .exports
                        .insert(specifier.exported.name().to_string(), node);
                }
            }
            Statement::ExportFromDeclaration(export) => {
                for specifier in &export.specifiers {
                    model.exports.insert(
                        specifier.exported.name().to_string(),
                        Node::Import {
                            source: export.source.value.to_string(),
                            export: Some(specifier.local.name().to_string()),
                        },
                    );
                }
            }
            Statement::ExportAllDeclaration(export) => match &export.exported {
                Some(name) => {
                    model.exports.insert(
                        name.name().to_string(),
                        Node::Import {
                            source: export.source.value.to_string(),
                            export: None,
                        },
                    );
                }
                None => model.stars.push(export.source.value.to_string()),
            },
            Statement::ExportDefaultDeclaration(export) => {
                let node = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        syntax::function_type(function, scoping)
                    }
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface) => interface
                        .id
                        .symbol_id
                        .get()
                        .map_or(Node::UNKNOWN, Node::Symbol),
                    value => value.as_expression().map_or(Node::UNKNOWN, |value| {
                        let node = Node::Expression(value.span());
                        match value {
                            oxc_ast::ast::Expression::Identifier(_) => node,
                            _ => Node::Primitive(Box::new(node)),
                        }
                    }),
                };
                model.exports.insert("default".to_string(), node);
            }
            _ => {}
        }
    }
}

fn bind(identifier: &BindingIdentifier<'_>, model: &mut Model) {
    model.exports.insert(
        identifier.name.to_string(),
        identifier
            .symbol_id
            .get()
            .map_or(Node::UNKNOWN, Node::Symbol),
    );
}
