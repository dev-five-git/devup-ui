use oxc_ast::ast::{
    Argument, CallExpression, Expression, IdentifierReference, JSXElementName, Statement, TSType,
    TaggedTemplateExpression,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::Bindings;

/// The binding of every read in what it visits, by where the read was written
pub struct Remember<'s> {
    bindings: &'s Bindings,
    recovered: FxHashMap<(u32, u32), SymbolId>,
}

impl<'s> Remember<'s> {
    pub fn new(bindings: &'s Bindings) -> Self {
        Self {
            bindings,
            recovered: FxHashMap::default(),
        }
    }

    pub fn into_recovered(self) -> FxHashMap<(u32, u32), SymbolId> {
        self.recovered
    }
}

impl<'a> Visit<'a> for Remember<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = self.bindings.symbol(it) {
            self.recovered.insert((it.span.start, it.span.end), symbol);
        }
    }
}

/// The binding each name read stands for, none when its reads differ
pub struct Names<'s> {
    bindings: &'s Bindings,
    names: FxHashMap<String, Option<SymbolId>>,
}

impl<'s> Names<'s> {
    pub fn new(bindings: &'s Bindings) -> Self {
        Self {
            bindings,
            names: FxHashMap::default(),
        }
    }

    pub fn into_visible(self) -> FxHashMap<String, SymbolId> {
        self.names
            .into_iter()
            .filter_map(|(name, symbol)| Some((name, symbol?)))
            .collect()
    }
}

impl<'a> Visit<'a> for Names<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = self.bindings.symbol(it);
        self.names
            .entry(it.name.to_string())
            .and_modify(|seen| {
                if *seen != symbol {
                    *seen = None;
                }
            })
            .or_insert(symbol);
    }
}

/// The reads a program keeps, outside the types it erases, of bindings whose
/// declarations the build removes
pub struct CompiledReads<'s> {
    bindings: &'s Bindings,
    /// Whether the calls and elements the build compiles are not reads: the
    /// code is read before they are
    before_lowering: bool,
    found: Vec<(u32, String)>,
}

impl<'s> CompiledReads<'s> {
    pub const fn new(bindings: &'s Bindings, before_lowering: bool) -> Self {
        Self {
            bindings,
            before_lowering,
            found: Vec::new(),
        }
    }

    pub fn into_found(self) -> Vec<(u32, String)> {
        self.found
    }

    /// The arguments of `call`, a call the build compiles, but not the base
    /// `styled(Box)` takes
    fn compiled_call(&mut self, call: &CallExpression<'_>) {
        let base = self.bindings.styles(&call.callee);
        let metadata = matches!(&call.callee, Expression::StaticMemberExpression(member)
            if member.property.name == "withConfig" && self.bindings.compiles(&member.object));
        for (index, argument) in call.arguments.iter().enumerate() {
            let renders = argument
                .as_expression()
                .is_some_and(|expression| self.bindings.kind(expression).is_some());
            if !(metadata || base && index == 0 && renders) {
                self.visit_argument(argument);
            }
        }
        self.compiled_target(&call.callee);
    }

    /// What `callee`, the target of a call the build compiles, is called on
    fn compiled_target(&mut self, callee: &Expression<'_>) {
        match callee {
            Expression::CallExpression(call) => self.compiled_call(call),
            Expression::StaticMemberExpression(member) => self.compiled_target(&member.object),
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for CompiledReads<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self
            .bindings
            .symbol(it)
            .is_some_and(|symbol| self.bindings.compiled.contains(&symbol))
        {
            self.found.push((it.span.start, it.name.to_string()));
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let props = it.arguments.get(1);
        let builds_element = self
            .bindings
            .jsx_function(&it.callee)
            .is_some_and(|function| match function.as_str() {
                "createElement" => props.is_none_or(Argument::is_expression),
                "jsx" | "jsxs" | "jsxDEV" => props.is_some_and(Argument::is_expression),
                _ => false,
            });
        if self.before_lowering
            && builds_element
            && it
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .is_some_and(|component| self.bindings.kind(component).is_some())
        {
            self.visit_expression(&it.callee);
            for argument in it.arguments.iter().skip(1) {
                self.visit_argument(argument);
            }
        } else if self.before_lowering && self.bindings.compiles(&it.callee) {
            self.compiled_call(it);
        } else {
            walk::walk_call_expression(self, it);
        }
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if self.before_lowering && self.bindings.compiles(&it.tag) {
            self.compiled_target(&it.tag);
            self.visit_template_literal(&it.quasi);
        } else {
            walk::walk_tagged_template_expression(self, it);
        }
    }

    fn visit_jsx_element_name(&mut self, it: &JSXElementName<'a>) {
        let rendered = matches!(it, JSXElementName::IdentifierReference(identifier)
            if self.bindings.renders(identifier));
        if !(self.before_lowering && rendered) {
            walk::walk_jsx_element_name(self, it);
        }
    }

    fn visit_ts_type(&mut self, _: &TSType<'a>) {}
}

/// Whether `program` calls `require` for `package`
pub fn requires(program: &oxc_ast::ast::Program<'_>, package: &str) -> bool {
    let mut requires = Requires {
        package,
        found: false,
    };
    requires.visit_program(program);
    requires.found
}

struct Requires<'p> {
    package: &'p str,
    found: bool,
}

impl<'a> Visit<'a> for Requires<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.found |= matches!(&it.callee, Expression::Identifier(callee) if callee.name == "require")
            && matches!(it.arguments.as_slice(), [Argument::StringLiteral(source)] if source.value == self.package);
        walk::walk_call_expression(self, it);
    }
}

/// Removes the declarations of aliases, wherever they stand
pub struct RemoveAliases<'s> {
    aliases: &'s FxHashSet<SymbolId>,
}

impl<'s> RemoveAliases<'s> {
    pub const fn new(aliases: &'s FxHashSet<SymbolId>) -> Self {
        Self { aliases }
    }
}

impl<'a> VisitMut<'a> for RemoveAliases<'_> {
    fn visit_statements(&mut self, it: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
        walk_mut::walk_statements(self, it);
        it.retain_mut(|statement| {
            let Statement::VariableDeclaration(declaration) = statement else {
                return true;
            };
            declaration.declarations.retain(|declarator| {
                !declarator
                    .id
                    .get_binding_identifier()
                    .and_then(|id| id.symbol_id.get())
                    .is_some_and(|symbol| self.aliases.contains(&symbol))
            });
            !declaration.declarations.is_empty()
        });
    }
}
