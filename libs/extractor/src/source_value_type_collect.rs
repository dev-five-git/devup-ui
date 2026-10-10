use oxc_ast::ast::{
    BindingPattern, Expression, FormalParameter, Function, ImportDeclaration,
    ImportDeclarationSpecifier, Program, TSInterfaceDeclaration, TSTypeAliasDeclaration,
    VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

use super::{
    annotation, exports,
    model::{Model, Node},
    syntax,
};

pub(super) fn model(program: &Program<'_>) -> Model {
    let scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut collector = Collector {
        scoping: &scoping,
        model: Model::default(),
    };
    collector.visit_program(program);
    exports::collect(program, &scoping, &mut collector.model);
    collector.model
}

struct Collector<'s> {
    scoping: &'s Scoping,
    model: Model,
}

impl Collector<'_> {
    fn bind(&mut self, pattern: &BindingPattern<'_>, node: Node, inferred: bool) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                if let Some(symbol) = identifier.symbol_id.get() {
                    let node = if inferred {
                        Node::Primitive(Box::new(node))
                    } else {
                        node
                    };
                    self.model.bindings.insert(symbol, node);
                }
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    let selected = property
                        .key
                        .static_name()
                        .map_or(Node::UNKNOWN, |key| node.clone().member(key.to_string()));
                    self.bind(&property.value, selected, inferred);
                }
                if let Some(rest) = &object.rest {
                    self.bind(&rest.argument, Node::UNKNOWN, false);
                }
            }
            BindingPattern::ArrayPattern(array) => {
                for (index, item) in array.elements.iter().enumerate() {
                    if let Some(item) = item {
                        self.bind(item, node.clone().member(index.to_string()), inferred);
                    }
                }
                if let Some(rest) = &array.rest {
                    self.bind(&rest.argument, Node::UNKNOWN, false);
                }
            }
            BindingPattern::AssignmentPattern(assignment) => self.bind(
                &assignment.left,
                Node::Union(vec![node, Node::Expression(assignment.right.span())]),
                inferred,
            ),
        }
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if let Expression::Identifier(identifier) = expression
            && matches!(syntax::reference(identifier, self.scoping), Node::Symbol(_))
        {
            self.model
                .bound_references
                .insert(identifier.span.start, identifier.name.to_string());
        }
        self.model.expressions.insert(
            expression.span(),
            syntax::expression(expression, self.scoping),
        );
        walk::walk_expression(self, expression);
    }

    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        let annotated = declaration.type_annotation.as_ref();
        let immutable = declaration
            .id
            .get_binding_identifiers()
            .iter()
            .all(|identifier| {
                identifier
                    .symbol_id
                    .get()
                    .is_some_and(|symbol| self.scoping.symbol_flags(symbol).is_const_variable())
            });
        let node = match annotated {
            Some(annotation) => annotation::ty(&annotation.type_annotation, self.scoping),
            None if immutable => declaration
                .init
                .as_ref()
                .map_or(Node::UNKNOWN, |value| Node::Expression(value.span())),
            None => Node::UNKNOWN,
        };
        self.bind(&declaration.id, node, annotated.is_none());
        walk::walk_variable_declarator(self, declaration);
    }

    fn visit_formal_parameter(&mut self, parameter: &FormalParameter<'a>) {
        let node = if parameter.optional {
            Node::UNKNOWN
        } else {
            parameter
                .type_annotation
                .as_ref()
                .map_or(Node::UNKNOWN, |annotation| {
                    annotation::ty(&annotation.type_annotation, self.scoping)
                })
        };
        self.bind(&parameter.pattern, node, false);
        walk::walk_formal_parameter(self, parameter);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        if let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) {
            let node = if self.scoping.symbol_is_mutated(symbol) {
                Node::UNKNOWN
            } else {
                syntax::function_type(function, self.scoping)
            };
            self.model
                .bindings
                .entry(symbol)
                .and_modify(|previous| {
                    *previous = Node::Union(vec![previous.clone(), node.clone()]);
                })
                .or_insert(node);
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_ts_type_alias_declaration(&mut self, alias: &TSTypeAliasDeclaration<'a>) {
        if let Some(symbol) = alias.id.symbol_id.get() {
            let node = if alias.type_parameters.is_some() {
                Node::UNKNOWN
            } else {
                annotation::ty(&alias.type_annotation, self.scoping)
            };
            self.model.bindings.insert(symbol, node);
        }
        walk::walk_ts_type_alias_declaration(self, alias);
    }

    fn visit_ts_interface_declaration(&mut self, interface: &TSInterfaceDeclaration<'a>) {
        if let Some(symbol) = interface.id.symbol_id.get() {
            let node = if interface.type_parameters.is_some() {
                Node::UNKNOWN
            } else {
                let mut parts: Vec<_> = interface
                    .extends
                    .iter()
                    .map(|base| {
                        if base.type_arguments.is_some() {
                            Node::UNKNOWN
                        } else {
                            annotation::name(&base.type_name, self.scoping)
                        }
                    })
                    .collect();
                parts.push(annotation::members(&interface.body.body, self.scoping));
                Node::Merge(parts)
            };
            self.model
                .bindings
                .entry(symbol)
                .and_modify(|previous| {
                    *previous = Node::Merge(vec![previous.clone(), node.clone()]);
                })
                .or_insert(node);
        }
        walk::walk_ts_interface_declaration(self, interface);
    }

    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        for specifier in import.specifiers.iter().flatten() {
            if let Some(symbol) = specifier.local().symbol_id.get() {
                let export = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(named) => {
                        Some(named.imported.name().to_string())
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        Some("default".to_string())
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => None,
                };
                self.model.bindings.insert(
                    symbol,
                    Node::Import {
                        source: import.source.value.to_string(),
                        export,
                    },
                );
            }
        }
    }
}
