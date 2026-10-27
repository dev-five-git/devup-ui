use oxc_allocator::{Address, UnstableAddress};
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Function, Program,
    Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::Visit;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{traversal::Collector, writes::Writes};

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Proof {
    Declaration(String, Address),
    Function,
    Alias(String),
    Other,
}

impl Proof {
    pub fn expression(node: &Expression<'_>) -> Self {
        match node {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
                Self::Function
            }
            Expression::Identifier(id) => Self::Alias(id.name.to_string()),
            _ => Self::Other,
        }
    }

    fn function(node: &Function<'_>) -> Self {
        Self::Declaration(
            node.id
                .as_ref()
                .map_or("", |id| id.name.as_str())
                .to_owned(),
            node.unstable_address(),
        )
    }

    pub fn default_declaration(node: &ExportDefaultDeclarationKind<'_>) -> Self {
        match node {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => Self::function(function),
            _ => node.as_expression().map_or(Self::Other, Self::expression),
        }
    }
}

#[derive(Default)]
pub(super) struct Callables {
    pub locals: FxHashMap<String, Proof>,
    pub default: Option<Proof>,
}

impl Callables {
    pub fn prepare(&mut self, program: &Program<'_>) {
        for statement in &program.body {
            match statement {
                Statement::FunctionDeclaration(node) => self.function(node),
                Statement::VariableDeclaration(node) => self.variables(node),
                Statement::ExportDeclaration(node) => match &node.declaration {
                    Declaration::FunctionDeclaration(node) => self.function(node),
                    Declaration::VariableDeclaration(node) => self.variables(node),
                    _ => {}
                },
                Statement::ExportDefaultDeclaration(node) => {
                    if let ExportDefaultDeclarationKind::FunctionDeclaration(function) =
                        &node.declaration
                    {
                        self.function(function);
                    }
                }
                _ => {}
            }
        }
    }

    fn function(&mut self, node: &Function<'_>) {
        let proof = Proof::function(node);
        let name = node.id.as_ref().map_or("", |id| id.name.as_str());
        self.locals.insert(name.to_owned(), proof);
    }

    fn variables(&mut self, node: &VariableDeclaration<'_>) {
        if node.kind != VariableDeclarationKind::Const {
            return;
        }
        for declaration in &node.declarations {
            if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                (&declaration.id, &declaration.init)
            {
                self.locals
                    .insert(id.name.to_string(), Proof::expression(init));
            }
        }
    }

    fn callable(&self) -> bool {
        let mut current = self.default.as_ref();
        let mut seen = FxHashSet::default();
        while let Some(proof) = current {
            match proof {
                Proof::Declaration(name, _) => {
                    return name.is_empty() || self.locals.get(name) == Some(proof);
                }
                Proof::Function => return true,
                Proof::Alias(name) => {
                    if !seen.insert(name) {
                        return false;
                    }
                    current = self.locals.get(name);
                }
                Proof::Other => return false,
            }
        }
        false
    }
}

impl Collector {
    pub fn finish(&mut self, program: &Program<'_>) {
        if let Some(callables) = &mut self.callables {
            Writes {
                scopes: &self.scopes,
                callables,
            }
            .visit_program(program);
            if let Some(observation) = &mut self.facts.observation {
                observation.default_callable = callables.callable();
            }
        }
    }
}
