//! Imported constants inlined where styles read them, so a value another
//! module declares with `const` becomes a static class instead of a CSS
//! variable set at runtime.

use std::collections::BTreeSet;
use std::rc::Rc;

use oxc_allocator::{Allocator, FromIn, GetAllocator};
use oxc_ast::ast::{
    Expression, ImportDeclarationSpecifier, JSXAttributeItem, JSXElementName, ObjectPropertyKind,
    Program, Statement, Str, VariableDeclarationKind,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{Visit, VisitMut, walk_mut};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{SPAN, SourceType};
use oxc_syntax::number::NumberBase;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{ModuleResolver, utils::get_str_by_property_key};

#[derive(Clone, Debug)]
enum Constant {
    String(String),
    Number(f64),
    Object(Rc<FxHashMap<String, Constant>>),
}

enum Imported {
    Named(String),
    Namespace,
}

/// Inline the primitive constants `program` imports and reads in styles,
/// returning the files read
pub(crate) fn inline_imported_constants<'a>(
    ast_builder: &AstBuilder<'a>,
    program: &mut Program<'a>,
    filename: &str,
    package: &str,
    resolver: &ModuleResolver,
) -> BTreeSet<String> {
    let mut imports: FxHashMap<&str, (&str, Imported)> = FxHashMap::default();
    let mut style_roots = FxHashSet::default();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let source = import.source.value.as_str();
        for specifier in import.specifiers.iter().flatten() {
            let local = specifier.local().name.as_str();
            if source.starts_with(package) {
                style_roots.insert(local);
                continue;
            }
            let imported = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    Imported::Named(specifier.imported.name().to_string())
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                    Imported::Named("default".to_string())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => Imported::Namespace,
            };
            imports.insert(local, (source, imported));
        }
    }
    if imports.is_empty() || style_roots.is_empty() {
        return BTreeSet::new();
    }
    let mut read = StyleReads {
        style_roots: &style_roots,
        names: FxHashSet::default(),
        depth: 0,
    };
    read.visit_program(program);
    let mut modules = Modules {
        resolver,
        exports: FxHashMap::default(),
        loading: Vec::new(),
    };
    let mut constants: FxHashMap<String, Constant> = FxHashMap::default();
    for name in read.names {
        let Some((source, imported)) = imports.get(name.as_str()) else {
            continue;
        };
        let Some(exports) = modules.exports(source, filename) else {
            continue;
        };
        let constant = match imported {
            Imported::Named(export) => exports.get(export).cloned(),
            Imported::Namespace => Some(Constant::Object(exports)),
        };
        if let Some(constant) = constant {
            constants.insert(name, constant);
        }
    }
    let dependencies = modules.exports.into_keys().collect();
    if constants.is_empty() {
        return dependencies;
    }
    let scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut symbols: FxHashMap<SymbolId, Constant> = FxHashMap::default();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            for specifier in import.specifiers.iter().flatten() {
                let local = specifier.local();
                if let (Some(symbol), Some(constant)) =
                    (local.symbol_id.get(), constants.get(local.name.as_str()))
                {
                    symbols.insert(symbol, constant.clone());
                }
            }
        }
    }
    Inline {
        ast_builder,
        scoping: &scoping,
        symbols: &symbols,
    }
    .visit_program(program);
    dependencies
}

/// Names read inside the props of the package's components and the arguments
/// of its functions
struct StyleReads<'s> {
    style_roots: &'s FxHashSet<&'s str>,
    names: FxHashSet<String>,
    depth: usize,
}

impl StyleReads<'_> {
    fn is_style_root(&self, expression: &Expression<'_>) -> bool {
        match expression {
            Expression::Identifier(identifier) => {
                self.style_roots.contains(identifier.name.as_str())
            }
            Expression::StaticMemberExpression(member) => self.is_style_root(&member.object),
            Expression::CallExpression(call) => self.is_style_root(&call.callee),
            _ => false,
        }
    }

    fn reading<T>(&mut self, style: bool, walk: impl FnOnce(&mut Self) -> T) -> T {
        self.depth += usize::from(style);
        let result = walk(self);
        self.depth -= usize::from(style);
        result
    }
}

impl<'a> Visit<'a> for StyleReads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        if self.depth > 0 {
            self.names.insert(identifier.name.to_string());
        }
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        self.visit_expression(&call.callee);
        let style = self.is_style_root(&call.callee);
        self.reading(style, |reads| {
            for argument in &call.arguments {
                reads.visit_argument(argument);
            }
        });
    }

    fn visit_tagged_template_expression(
        &mut self,
        tagged: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        self.visit_expression(&tagged.tag);
        let style = self.is_style_root(&tagged.tag);
        self.reading(style, |reads| reads.visit_template_literal(&tagged.quasi));
    }

    fn visit_jsx_opening_element(&mut self, element: &oxc_ast::ast::JSXOpeningElement<'a>) {
        let style = match &element.name {
            JSXElementName::IdentifierReference(identifier) => {
                self.style_roots.contains(identifier.name.as_str())
            }
            JSXElementName::MemberExpression(member) => {
                let mut object = &member.object;
                while let oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(inner) = object
                {
                    object = &inner.object;
                }
                matches!(object, oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier)
                    if self.style_roots.contains(identifier.name.as_str()))
            }
            _ => false,
        };
        self.reading(style, |reads| {
            for attribute in &element.attributes {
                match attribute {
                    JSXAttributeItem::Attribute(attribute) => {
                        if let Some(value) = &attribute.value {
                            reads.visit_jsx_attribute_value(value);
                        }
                    }
                    JSXAttributeItem::SpreadAttribute(spread) => {
                        reads.visit_expression(&spread.argument);
                    }
                }
            }
        });
    }
}

/// The constant exports of the modules read, by path
struct Modules<'r> {
    resolver: &'r ModuleResolver,
    exports: FxHashMap<String, Rc<FxHashMap<String, Constant>>>,
    loading: Vec<String>,
}

impl Modules<'_> {
    fn exports(
        &mut self,
        specifier: &str,
        importer: &str,
    ) -> Option<Rc<FxHashMap<String, Constant>>> {
        let module = (self.resolver)(specifier, importer)?;
        if let Some(exports) = self.exports.get(&module.path) {
            return Some(exports.clone());
        }
        if self.loading.contains(&module.path) {
            return None;
        }
        self.loading.push(module.path.clone());
        let exports = Rc::new(self.read(&module.path, &module.code));
        self.loading.pop();
        self.exports.insert(module.path, exports.clone());
        Some(exports)
    }

    fn read(&mut self, path: &str, code: &str) -> FxHashMap<String, Constant> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
        let program = Parser::new(&allocator, code, source_type).parse().program;
        let mut scope = ModuleScope {
            path,
            locals: FxHashMap::default(),
            imports: FxHashMap::default(),
        };
        let mut exports = FxHashMap::default();
        for statement in &program.body {
            match statement {
                Statement::ImportDeclaration(import) => {
                    for specifier in import.specifiers.iter().flatten() {
                        let imported = match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                                Imported::Named(specifier.imported.name().to_string())
                            }
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                                Imported::Named("default".to_string())
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                                Imported::Namespace
                            }
                        };
                        scope.imports.insert(
                            specifier.local().name.to_string(),
                            (import.source.value.to_string(), imported),
                        );
                    }
                }
                Statement::VariableDeclaration(declaration) => {
                    scope.declare(self, declaration, None);
                }
                Statement::ExportDeclaration(export) => {
                    if let oxc_ast::ast::Declaration::VariableDeclaration(declaration) =
                        &export.declaration
                    {
                        scope.declare(self, declaration, Some(&mut exports));
                    }
                }
                Statement::ExportNamedDeclaration(export) => {
                    for specifier in &export.specifiers {
                        if let Some(constant) = scope.lookup(self, &specifier.local.name()) {
                            exports.insert(specifier.exported.name().to_string(), constant);
                        }
                    }
                }
                Statement::ExportFromDeclaration(export) => {
                    if let Some(from) = self.exports(&export.source.value, path) {
                        for specifier in &export.specifiers {
                            if let Some(constant) = from.get(specifier.local.name().as_str()) {
                                exports.insert(
                                    specifier.exported.name().to_string(),
                                    constant.clone(),
                                );
                            }
                        }
                    }
                }
                Statement::ExportAllDeclaration(export) => {
                    if let Some(from) = self.exports(&export.source.value, path) {
                        match &export.exported {
                            Some(exported) => {
                                exports.insert(exported.name().to_string(), Constant::Object(from));
                            }
                            None => {
                                for (name, constant) in from.iter() {
                                    if name != "default" {
                                        exports
                                            .entry(name.clone())
                                            .or_insert_with(|| constant.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                Statement::ExportDefaultDeclaration(export) => {
                    if let Some(expression) = export.declaration.as_expression()
                        && let Some(constant) = scope.evaluate(self, expression)
                    {
                        exports.insert("default".to_string(), constant);
                    }
                }
                _ => {}
            }
        }
        exports
    }
}

/// The top-level constants and imports of a module being read
struct ModuleScope<'p> {
    path: &'p str,
    locals: FxHashMap<String, Constant>,
    imports: FxHashMap<String, (String, Imported)>,
}

impl ModuleScope<'_> {
    fn declare(
        &mut self,
        modules: &mut Modules<'_>,
        declaration: &oxc_ast::ast::VariableDeclaration<'_>,
        mut exports: Option<&mut FxHashMap<String, Constant>>,
    ) {
        if declaration.kind != VariableDeclarationKind::Const {
            return;
        }
        for declarator in &declaration.declarations {
            if let Some(name) = declarator.id.get_identifier_name()
                && let Some(init) = &declarator.init
                && let Some(constant) = self.evaluate(modules, init)
            {
                if let Some(exports) = exports.as_deref_mut() {
                    exports.insert(name.to_string(), constant.clone());
                }
                self.locals.insert(name.to_string(), constant);
            }
        }
    }

    fn lookup(&self, modules: &mut Modules<'_>, name: &str) -> Option<Constant> {
        if let Some(constant) = self.locals.get(name) {
            return Some(constant.clone());
        }
        let (source, imported) = self.imports.get(name)?;
        let exports = modules.exports(source, self.path)?;
        match imported {
            Imported::Named(export) => exports.get(export).cloned(),
            Imported::Namespace => Some(Constant::Object(exports)),
        }
    }

    fn evaluate(&self, modules: &mut Modules<'_>, expression: &Expression<'_>) -> Option<Constant> {
        match expression {
            Expression::StringLiteral(literal) => Some(Constant::String(literal.value.to_string())),
            Expression::NumericLiteral(literal) => Some(Constant::Number(literal.value)),
            Expression::UnaryExpression(unary)
                if unary.operator == oxc_syntax::operator::UnaryOperator::UnaryNegation =>
            {
                match self.evaluate(modules, &unary.argument)? {
                    Constant::Number(number) => Some(Constant::Number(-number)),
                    _ => None,
                }
            }
            Expression::TemplateLiteral(template) => template
                .single_quasi()
                .map(|quasi| Constant::String(quasi.to_string())),
            Expression::ObjectExpression(object) => {
                let mut properties = FxHashMap::default();
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && !property.computed
                        && let Some(key) = get_str_by_property_key(&property.key)
                        && let Some(value) = self.evaluate(modules, &property.value)
                    {
                        properties.insert(key.to_string(), value);
                    }
                }
                Some(Constant::Object(Rc::new(properties)))
            }
            Expression::Identifier(identifier) => self.lookup(modules, &identifier.name),
            Expression::StaticMemberExpression(member) => {
                match self.evaluate(modules, &member.object)? {
                    Constant::Object(object) => object.get(member.property.name.as_str()).cloned(),
                    _ => None,
                }
            }
            Expression::TSAsExpression(inner) => self.evaluate(modules, &inner.expression),
            Expression::TSSatisfiesExpression(inner) => self.evaluate(modules, &inner.expression),
            Expression::ParenthesizedExpression(inner) => self.evaluate(modules, &inner.expression),
            _ => None,
        }
    }
}

/// Replaces reads of the imported constants that are primitives
struct Inline<'s, 'a> {
    ast_builder: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    symbols: &'s FxHashMap<SymbolId, Constant>,
}

impl<'a> Inline<'_, 'a> {
    fn constant(&self, expression: &Expression<'a>) -> Option<Constant> {
        match expression {
            Expression::Identifier(identifier) => {
                let reference = identifier.reference_id.get()?;
                let symbol = self.scoping.get_reference(reference).symbol_id()?;
                self.symbols.get(&symbol).cloned()
            }
            Expression::StaticMemberExpression(member) => match self.constant(&member.object)? {
                Constant::Object(object) => object.get(member.property.name.as_str()).cloned(),
                _ => None,
            },
            Expression::ComputedMemberExpression(member) => {
                let key = crate::utils::get_string_by_literal_expression(&member.expression)?;
                match self.constant(&member.object)? {
                    Constant::Object(object) => object.get(key.as_ref()).cloned(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn literal(&self, constant: &Constant) -> Option<Expression<'a>> {
        match constant {
            Constant::String(value) => Some(Expression::new_string_literal(
                SPAN,
                Str::from_in(value.as_str(), self.ast_builder.allocator()),
                None,
                self.ast_builder,
            )),
            Constant::Number(value) => Some(Expression::new_numeric_literal(
                SPAN,
                *value,
                None,
                NumberBase::Decimal,
                self.ast_builder,
            )),
            Constant::Object(_) => None,
        }
    }
}

impl<'a> VisitMut<'a> for Inline<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Some(literal) = self
            .constant(expression)
            .and_then(|constant| self.literal(&constant))
        {
            *expression = literal;
            return;
        }
        walk_mut::walk_expression(self, expression);
    }

    fn visit_object_property(&mut self, property: &mut oxc_ast::ast::ObjectProperty<'a>) {
        walk_mut::walk_object_property(self, property);
        if property.shorthand && !matches!(property.value, Expression::Identifier(_)) {
            property.shorthand = false;
        }
    }
}
