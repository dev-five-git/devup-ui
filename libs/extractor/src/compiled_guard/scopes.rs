use oxc_allocator::{Address, GetAddress};
use oxc_ast::{
    AstKind,
    ast::{
        BindingPattern, ClassType, FormalParameters, FunctionType, Program, VariableDeclarationKind,
    },
};
use oxc_ast_visit::Visit;
use rustc_hash::FxHashMap;

use super::wire::Origin;

pub(super) struct Scope {
    pub parent: Option<usize>,
    function: bool,
    pub bindings: FxHashMap<String, Option<Origin>>,
}

pub(super) struct Scopes {
    pub nodes: FxHashMap<Address, usize>,
    pub entries: Vec<Scope>,
    current: usize,
    parents: Vec<usize>,
}

impl Scopes {
    pub fn build(program: &Program<'_>) -> Self {
        let mut scopes = Self {
            nodes: FxHashMap::default(),
            entries: vec![Scope {
                parent: None,
                function: true,
                bindings: FxHashMap::default(),
            }],
            current: 0,
            parents: Vec::new(),
        };
        scopes.visit_program(program);
        scopes
    }

    pub fn at(&self, address: Address) -> usize {
        self.nodes.get(&address).copied().unwrap_or(0)
    }

    pub fn owner(&self, mut scope: usize, name: &str) -> Option<usize> {
        loop {
            if self.entries[scope].bindings.contains_key(name) {
                return Some(scope);
            }
            scope = self.entries[scope].parent?;
        }
    }

    pub fn binding(&self, scope: usize, name: &str) -> Option<Origin> {
        let owner = self.owner(scope, name)?;
        self.entries[owner].bindings.get(name)?.clone()
    }

    fn declare(&mut self, scope: usize, name: &str) {
        self.entries[scope].bindings.insert(name.to_owned(), None);
    }

    fn pattern(&mut self, scope: usize, pattern: &BindingPattern<'_>) {
        for name in pattern_names(pattern) {
            self.declare(scope, &name);
        }
    }

    fn parameters(&mut self, params: &FormalParameters<'_>) {
        for param in &params.items {
            self.pattern(self.current, &param.pattern);
        }
        if let Some(rest) = &params.rest {
            self.pattern(self.current, &rest.rest.argument);
        }
    }
}

impl<'a> Visit<'a> for Scopes {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let parent = self.current;
        self.parents.push(parent);
        let function = matches!(
            kind,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
        );
        let lexical = matches!(
            kind,
            AstKind::Class(_)
                | AstKind::BlockStatement(_)
                | AstKind::FunctionBody(_)
                | AstKind::CatchClause(_)
                | AstKind::ForStatement(_)
                | AstKind::ForOfStatement(_)
                | AstKind::ForInStatement(_)
                | AstKind::StaticBlock(_)
                | AstKind::SwitchStatement(_)
        );
        if function || lexical {
            self.current = self.entries.len();
            self.entries.push(Scope {
                parent: Some(parent),
                function,
                bindings: FxHashMap::default(),
            });
        }
        self.nodes.insert(kind.address(), self.current);
        match kind {
            AstKind::Function(node) => {
                if let Some(id) = &node.id {
                    let target = match node.r#type {
                        FunctionType::FunctionDeclaration => parent,
                        FunctionType::FunctionExpression
                        | FunctionType::TSDeclareFunction
                        | FunctionType::TSEmptyBodyFunctionExpression => self.current,
                    };
                    self.declare(target, id.name.as_str());
                }
                self.parameters(&node.params);
            }
            AstKind::ArrowFunctionExpression(node) => self.parameters(&node.params),
            AstKind::Class(node) => {
                if let Some(id) = &node.id {
                    if node.r#type == ClassType::ClassDeclaration {
                        self.declare(parent, id.name.as_str());
                    }
                    self.declare(self.current, id.name.as_str());
                }
            }
            AstKind::CatchClause(node) => {
                if let Some(param) = &node.param {
                    self.pattern(self.current, &param.pattern);
                }
            }
            AstKind::VariableDeclaration(node) => {
                let mut target = self.current;
                if node.kind == VariableDeclarationKind::Var {
                    while !self.entries[target].function
                        && let Some(parent) = self.entries[target].parent
                    {
                        target = parent;
                    }
                }
                for declaration in &node.declarations {
                    self.pattern(target, &declaration.id);
                }
            }
            _ => {}
        }
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        if let Some(parent) = self.parents.pop() {
            self.current = parent;
        }
    }
}

pub(super) fn pattern_names(pattern: &BindingPattern<'_>) -> Vec<String> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => vec![id.name.to_string()],
        BindingPattern::AssignmentPattern(node) => pattern_names(&node.left),
        BindingPattern::ObjectPattern(node) => {
            let mut names: Vec<_> = node
                .properties
                .iter()
                .flat_map(|property| pattern_names(&property.value))
                .collect();
            if let Some(rest) = &node.rest {
                names.extend(pattern_names(&rest.argument));
            }
            names
        }
        BindingPattern::ArrayPattern(node) => {
            let mut names: Vec<_> = node
                .elements
                .iter()
                .flatten()
                .flat_map(pattern_names)
                .collect();
            if let Some(rest) = &node.rest {
                names.extend(pattern_names(&rest.argument));
            }
            names
        }
    }
}
