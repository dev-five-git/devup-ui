//! Imported constants inlined where styles read them, so a value another
//! module declares with `const` becomes a static class instead of a CSS
//! variable set at runtime.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use oxc_allocator::{Allocator, FromIn, GetAllocator};
use oxc_ast::ast::{
    Argument, Expression, ImportDeclarationSpecifier, JSXAttributeItem, JSXElementName,
    ObjectPropertyKind, Program, Statement, Str, VariableDeclarationKind,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{SPAN, SourceType};
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::stylex::StylexFunction;
use crate::{ExtractOption, ModuleResolver, utils::get_str_by_property_key};

#[derive(Clone, Debug)]
enum Constant {
    String(String),
    Number(f64),
    Object(Rc<FxHashMap<String, Constant>>),
    /// `StyleX` custom properties by key, which read as their `var()`
    Vars(Rc<FxHashMap<String, String>>),
    /// The class a `StyleX` theme applies
    Theme(String),
}

/// What inlining found: the files read, and the `StyleX` values imported from
/// other modules, by the name the program binds them to
#[derive(Default)]
pub(crate) struct Inlined {
    pub dependencies: BTreeSet<String>,
    pub stylex_vars: FxHashMap<String, FxHashMap<String, String>>,
    pub stylex_themes: FxHashMap<String, String>,
}

enum Imported {
    Named(String),
    Namespace,
}

/// Inline the primitive constants `program` reads in styles, its own
/// module-level `const`s and those it imports, returning the files read
pub(crate) fn inline_constants<'a>(
    ast_builder: &AstBuilder<'a>,
    program: &mut Program<'a>,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
) -> Inlined {
    let is_style_package =
        |source: &str| source.starts_with(&option.package) || source == crate::STYLEX_PACKAGE;
    let mut style_roots = FxHashSet::default();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && is_style_package(&import.source.value)
        {
            for specifier in import.specifiers.iter().flatten() {
                style_roots.insert(specifier.local().name.as_str());
            }
        }
    }
    if style_roots.is_empty() {
        return Inlined::default();
    }
    let mut read = StyleReads {
        style_roots: &style_roots,
        names: FxHashSet::default(),
        depth: 0,
    };
    read.visit_program(program);
    if read.names.is_empty() {
        return Inlined::default();
    }
    let mut modules = Modules {
        resolver,
        option,
        exports: FxHashMap::default(),
        loading: Vec::new(),
    };
    let mut symbols: FxHashMap<SymbolId, Constant> = FxHashMap::default();
    let mut inlined = Inlined::default();
    let (scoping, reads_math) = {
        let mut scope = ModuleScope::new(filename);
        let mut bindings: FxHashMap<&str, Vec<&Cell<Option<SymbolId>>>> = FxHashMap::default();
        for statement in &program.body {
            let declaration = match statement {
                Statement::ImportDeclaration(import) => {
                    if !is_style_package(&import.source.value) {
                        scope.import(import);
                        for specifier in import.specifiers.iter().flatten() {
                            let local = specifier.local();
                            bindings
                                .entry(local.name.as_str())
                                .or_default()
                                .push(&local.symbol_id);
                        }
                    }
                    continue;
                }
                Statement::VariableDeclaration(declaration) => declaration,
                Statement::ExportDeclaration(export) => {
                    let oxc_ast::ast::Declaration::VariableDeclaration(declaration) =
                        &export.declaration
                    else {
                        continue;
                    };
                    declaration
                }
                _ => continue,
            };
            scope.declare(declaration);
            for declarator in &declaration.declarations {
                if let oxc_ast::ast::BindingPattern::BindingIdentifier(identifier) = &declarator.id
                {
                    bindings
                        .entry(identifier.name.as_str())
                        .or_default()
                        .push(&identifier.symbol_id);
                }
            }
        }
        // Scoping is only worth building when a style reads a name that may
        // hold a constant
        let reads_math = read.names.contains("Math") && !scope.binds("Math");
        if !reads_math && !read.names.iter().any(|name| scope.binds(name)) {
            return Inlined::default();
        }
        let scoping = SemanticBuilder::new()
            .build(program)
            .semantic
            .into_scoping();
        for name in &read.names {
            if let Some(constant) = scope.lookup(&mut modules, name) {
                match &constant {
                    Constant::Vars(vars) => {
                        inlined
                            .stylex_vars
                            .insert(name.clone(), vars.as_ref().clone());
                    }
                    Constant::Theme(class) => {
                        inlined.stylex_themes.insert(name.clone(), class.clone());
                    }
                    _ => {}
                }
                for symbol in bindings.get(name.as_str()).into_iter().flatten() {
                    symbols.extend(symbol.get().map(|symbol| (symbol, constant.clone())));
                }
            }
        }
        (scoping, reads_math)
    };
    inlined.dependencies = modules.exports.into_keys().collect();
    if !symbols.is_empty() || reads_math {
        Inline {
            ast_builder,
            scoping: &scoping,
            symbols: &symbols,
        }
        .visit_program(program);
    }
    inlined
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
    resolver: Option<&'r ModuleResolver>,
    option: &'r ExtractOption,
    exports: FxHashMap<String, Rc<FxHashMap<String, Constant>>>,
    loading: Vec<String>,
}

impl Modules<'_> {
    fn exports(
        &mut self,
        specifier: &str,
        importer: &str,
    ) -> Option<Rc<FxHashMap<String, Constant>>> {
        let module = (self.resolver?)(specifier, importer)?;
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
        let mut scope = ModuleScope::new(path);
        let mut exports = FxHashMap::default();
        let mut exported: Vec<(String, String)> = Vec::new();
        let mut commonjs = CommonJs::new(&program);
        for statement in &program.body {
            match statement {
                Statement::ImportDeclaration(import) => scope.import(import),
                Statement::VariableDeclaration(declaration) => {
                    scope.require(declaration);
                    scope.declare(declaration);
                }
                Statement::ExpressionStatement(statement) => {
                    commonjs.assign(&mut scope, self, &statement.expression);
                }
                Statement::ExportDeclaration(export) => {
                    if let oxc_ast::ast::Declaration::VariableDeclaration(declaration) =
                        &export.declaration
                    {
                        for name in scope.declare(declaration) {
                            exported.push((name.clone(), name));
                        }
                    }
                }
                Statement::ExportNamedDeclaration(export) => {
                    for specifier in &export.specifiers {
                        exported.push((
                            specifier.exported.name().to_string(),
                            specifier.local.name().to_string(),
                        ));
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
        for (exported, local) in exported {
            if let Some(constant) = scope.lookup(self, &local) {
                exports.insert(exported, constant);
            }
        }
        commonjs.finish(&mut exports);
        exports
    }
}
/// The exports of a `CommonJS` module. A property counts only when the module
/// assigns it once, as anything assigned again may change after it is read.
#[derive(Default)]
struct CommonJs {
    /// Assignments to each exported property anywhere in the module, `*` for
    /// `module.exports` itself; `void 0` placeholders do not count
    writes: FxHashMap<String, usize>,
    exports: FxHashMap<String, Constant>,
    whole: Option<Constant>,
    es_module: bool,
    seen: bool,
}

impl CommonJs {
    fn new(program: &Program<'_>) -> Self {
        let mut commonjs = Self::default();
        commonjs.visit_program(program);
        commonjs
    }

    fn assign(
        &mut self,
        scope: &mut ModuleScope<'_, '_>,
        modules: &mut Modules<'_>,
        expression: &Expression<'_>,
    ) {
        if let Expression::CallExpression(call) = expression
            && is_define_es_module(call)
        {
            self.es_module = true;
            return;
        }
        let Expression::AssignmentExpression(assignment) = expression else {
            return;
        };
        let Some(key) = assignment
            .left
            .as_simple_assignment_target()
            .and_then(|target| target.as_member_expression())
            .and_then(commonjs_target)
        else {
            return;
        };
        self.seen = true;
        if key == "__esModule" {
            self.es_module = true;
            return;
        }
        if self.writes.get(&key) != Some(&1) {
            return;
        }
        let Some(value) = scope.evaluate(modules, &assignment.right) else {
            return;
        };
        if key == "*" {
            self.whole = Some(value);
        } else {
            self.exports.insert(key, value);
        }
    }

    fn finish(self, exports: &mut FxHashMap<String, Constant>) {
        if !self.seen {
            return;
        }
        let mut properties = FxHashMap::default();
        if let Some(Constant::Object(object)) = &self.whole {
            for (key, value) in object.iter() {
                if !self.writes.contains_key(key) {
                    properties.insert(key.clone(), value.clone());
                }
            }
        }
        properties.extend(self.exports);
        let default = if self.es_module {
            properties.get("default").cloned()
        } else {
            match self.whole {
                Some(Constant::Object(_)) | None => Some(Constant::Object(Rc::new(
                    properties
                        .clone()
                        .into_iter()
                        .filter(|(key, _)| key != "default")
                        .collect(),
                ))),
                primitive => primitive,
            }
        };
        exports.extend(properties);
        if let Some(default) = default {
            exports.insert("default".to_string(), default);
        }
    }
}

impl<'a> Visit<'a> for CommonJs {
    fn visit_assignment_expression(&mut self, assignment: &oxc_ast::ast::AssignmentExpression<'a>) {
        let mut value = &assignment.right;
        while let Expression::AssignmentExpression(inner) = value {
            value = &inner.right;
        }
        let placeholder = matches!(value, Expression::UnaryExpression(unary)
            if unary.operator == oxc_syntax::operator::UnaryOperator::Void);
        if !placeholder
            && let Some(key) = assignment
                .left
                .as_simple_assignment_target()
                .and_then(|target| target.as_member_expression())
                .and_then(commonjs_target)
        {
            *self.writes.entry(key).or_default() += 1;
        }
        walk::walk_assignment_expression(self, assignment);
    }
}

/// `exports.x` / `module.exports.x` -> `x`, `module.exports` -> `*`
fn commonjs_target(member: &oxc_ast::ast::MemberExpression<'_>) -> Option<String> {
    let key = member.static_property_name()?;
    let object = member.object();
    if matches!(object, Expression::Identifier(identifier) if identifier.name == "module") {
        return (key == "exports").then(|| "*".to_string());
    }
    let is_exports = match object {
        Expression::Identifier(identifier) => identifier.name == "exports",
        Expression::StaticMemberExpression(inner) => {
            inner.property.name == "exports"
                && matches!(&inner.object, Expression::Identifier(identifier) if identifier.name == "module")
        }
        _ => false,
    };
    is_exports.then(|| key.to_string())
}

/// `Object.defineProperty(exports, '__esModule', ...)`
fn is_define_es_module(call: &oxc_ast::ast::CallExpression<'_>) -> bool {
    matches!(&call.callee, Expression::StaticMemberExpression(callee)
        if callee.property.name == "defineProperty"
            && matches!(&callee.object, Expression::Identifier(object) if object.name == "Object"))
        && matches!(call.arguments.first(), Some(Argument::Identifier(target)) if target.name == "exports")
        && matches!(call.arguments.get(1), Some(Argument::StringLiteral(key)) if key.value == "__esModule")
}

/// The top-level constants and imports of a module, each constant evaluated
/// when first read
struct ModuleScope<'p, 'a> {
    path: &'p str,
    locals: FxHashMap<String, Constant>,
    declarations: FxHashMap<String, &'p Expression<'a>>,
    imports: FxHashMap<String, (String, Imported)>,
}

impl<'p, 'a> ModuleScope<'p, 'a> {
    fn new(path: &'p str) -> Self {
        Self {
            path,
            locals: FxHashMap::default(),
            declarations: FxHashMap::default(),
            imports: FxHashMap::default(),
        }
    }

    fn binds(&self, name: &str) -> bool {
        self.declarations.contains_key(name) || self.imports.contains_key(name)
    }

    fn is_global_math(&self, expression: &Expression<'_>) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == "Math"
                && !self.binds("Math")
                && !self.locals.contains_key("Math"))
    }

    fn import(&mut self, import: &oxc_ast::ast::ImportDeclaration<'_>) {
        for specifier in import.specifiers.iter().flatten() {
            let imported = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    Imported::Named(specifier.imported.name().to_string())
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                    Imported::Named("default".to_string())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => Imported::Namespace,
            };
            self.imports.insert(
                specifier.local().name.to_string(),
                (import.source.value.to_string(), imported),
            );
        }
    }
    /// `const x = require('m')` and `const { a, b: c } = require('m')`
    fn require(&mut self, declaration: &oxc_ast::ast::VariableDeclaration<'_>) {
        for declarator in &declaration.declarations {
            let Some(Expression::CallExpression(call)) = &declarator.init else {
                continue;
            };
            let (Expression::Identifier(callee), [Argument::StringLiteral(source)]) =
                (&call.callee, call.arguments.as_slice())
            else {
                continue;
            };
            if callee.name != "require" {
                continue;
            }
            let source = source.value.to_string();
            match &declarator.id {
                oxc_ast::ast::BindingPattern::BindingIdentifier(identifier) => {
                    self.imports
                        .insert(identifier.name.to_string(), (source, Imported::Namespace));
                }
                oxc_ast::ast::BindingPattern::ObjectPattern(pattern) => {
                    for property in &pattern.properties {
                        if let Some(key) = property.key.static_name()
                            && let Some(local) = property.value.get_identifier_name()
                        {
                            self.imports.insert(
                                local.to_string(),
                                (source.clone(), Imported::Named(key.to_string())),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Record the `const`s of `declaration`, returning the names it binds
    fn declare(&mut self, declaration: &'p oxc_ast::ast::VariableDeclaration<'a>) -> Vec<String> {
        let mut names = Vec::new();
        if declaration.kind != VariableDeclarationKind::Const {
            return names;
        }
        for declarator in &declaration.declarations {
            if let Some(name) = declarator.id.get_identifier_name()
                && let Some(init) = &declarator.init
            {
                self.declarations.insert(name.to_string(), init);
                names.push(name.to_string());
            }
        }
        names
    }

    fn lookup(&mut self, modules: &mut Modules<'_>, name: &str) -> Option<Constant> {
        if let Some(constant) = self.locals.get(name) {
            return Some(constant.clone());
        }
        // Taken out while it is evaluated, so a constant reading itself stops
        if let Some(init) = self.declarations.remove(name)
            && let Some(constant) = self.evaluate(modules, init)
        {
            self.locals.insert(name.to_string(), constant.clone());
            return Some(constant);
        }
        let (source, imported) = self.imports.get(name)?;
        let exports = modules.exports(source, self.path)?;
        match imported {
            Imported::Named(export) => exports.get(export).cloned(),
            Imported::Namespace => Some(Constant::Object(exports)),
        }
    }

    fn stylex_function(&self, callee: &Expression<'_>) -> Option<StylexFunction> {
        let (name, member) = match callee {
            Expression::Identifier(identifier) => (identifier.name.as_str(), None),
            Expression::StaticMemberExpression(member) => match &member.object {
                Expression::Identifier(object) => {
                    (object.name.as_str(), Some(member.property.name.as_str()))
                }
                _ => return None,
            },
            _ => return None,
        };
        let export = match (self.imports.get(name)?, member) {
            ((source, _), _) if source != crate::STYLEX_PACKAGE => return None,
            ((_, Imported::Named(export)), None) => export.as_str(),
            ((_, Imported::Namespace), Some(export)) => export,
            ((_, Imported::Named(export)), Some(member)) if export == "default" => member,
            _ => return None,
        };
        StylexFunction::from_export_name(export)
    }

    /// A value `StyleX` gives when this module's own extraction reads it, with
    /// the names that extraction generates
    fn evaluate_stylex(
        &mut self,
        modules: &mut Modules<'_>,
        call: &oxc_ast::ast::CallExpression<'_>,
    ) -> Option<Constant> {
        let function = self.stylex_function(&call.callee)?;
        let split_filename = crate::css_bucket(self.path, modules.option);
        match (function, call.arguments.as_slice()) {
            (
                function @ (StylexFunction::DefineVars | StylexFunction::CreateThemeContract),
                [Argument::ObjectExpression(object)],
            ) => {
                let mut vars = FxHashMap::default();
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && let Some(key) = self.property_key(modules, &property.key)
                        && (function == StylexFunction::CreateThemeContract
                            || self.variable_value(modules, &property.value))
                    {
                        let variable = crate::stylex::define_vars_variable(
                            self.path,
                            &key,
                            split_filename.as_deref(),
                        );
                        vars.insert(key, variable);
                    }
                }
                Some(Constant::Vars(Rc::new(vars)))
            }
            (StylexFunction::DefineConsts, [Argument::ObjectExpression(object)]) => {
                let mut values = FxHashMap::default();
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && let Some(key) = self.property_key(modules, &property.key)
                        && let Some(value) = self.literal_text(modules, &property.value)
                    {
                        values.insert(key, Constant::String(value));
                    }
                }
                Some(Constant::Object(Rc::new(values)))
            }
            (
                StylexFunction::CreateTheme,
                [
                    Argument::Identifier(contract),
                    Argument::ObjectExpression(_),
                ],
            ) => matches!(self.lookup(modules, &contract.name)?, Constant::Vars(_)).then(|| {
                Constant::Theme(crate::stylex::create_theme_class(
                    self.path,
                    &contract.name,
                    split_filename.as_deref(),
                ))
            }),
            _ => None,
        }
    }

    fn property_key(
        &mut self,
        modules: &mut Modules<'_>,
        key: &oxc_ast::ast::PropertyKey<'_>,
    ) -> Option<String> {
        match crate::utils::get_string_by_property_key(key) {
            Some(key) => Some(key),
            None => self.literal_text(modules, key.as_expression()?),
        }
    }

    /// Whether the module's own extraction reads `value` as the value of a
    /// `StyleX` variable, as [`crate::stylex::variable_values`] reads it once
    /// constants are inlined
    fn variable_value(&mut self, modules: &mut Modules<'_>, value: &Expression<'_>) -> bool {
        let value = crate::stylex::unwrap_types_call(value);
        if matches!(value, Expression::NullLiteral(_))
            || self.literal_text(modules, value).is_some()
        {
            return true;
        }
        let Expression::ObjectExpression(object) = value else {
            return false;
        };
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            let Some(key) = self.property_key(modules, &property.key) else {
                return false;
            };
            if (key != "default" && css::at_rule::split_at_rule_key(&key).is_none())
                || !self.variable_value(modules, &property.value)
            {
                return false;
            }
        }
        true
    }

    /// `value` as the text the module's own extraction reads it as, when that
    /// extraction knows it
    fn literal_text(
        &mut self,
        modules: &mut Modules<'_>,
        value: &Expression<'_>,
    ) -> Option<String> {
        match crate::utils::get_string_by_literal_expression(value) {
            Some(text) => Some(text.into_owned()),
            None => js_string(&self.evaluate(modules, value)?),
        }
    }

    fn evaluate(
        &mut self,
        modules: &mut Modules<'_>,
        expression: &Expression<'_>,
    ) -> Option<Constant> {
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
            Expression::TemplateLiteral(template) => {
                let mut values = Vec::with_capacity(template.expressions.len());
                for expression in &template.expressions {
                    values.push(self.evaluate(modules, expression)?);
                }
                fold_template(template, &values)
            }
            Expression::BinaryExpression(binary) => {
                let left = self.evaluate(modules, &binary.left)?;
                let right = self.evaluate(modules, &binary.right)?;
                fold_binary(binary.operator, &left, &right)
            }
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
            Expression::StaticMemberExpression(member) if self.is_global_math(&member.object) => {
                math_constant(member.property.name.as_str())
            }
            Expression::StaticMemberExpression(member) => member_of(
                &self.evaluate(modules, &member.object)?,
                member.property.name.as_str(),
            ),
            Expression::CallExpression(call) => match &call.callee {
                Expression::StaticMemberExpression(callee)
                    if self.is_global_math(&callee.object) =>
                {
                    let mut arguments = Vec::with_capacity(call.arguments.len());
                    for argument in &call.arguments {
                        arguments.push(self.evaluate(modules, argument.as_expression()?)?);
                    }
                    fold_math(callee.property.name.as_str(), &arguments)
                }
                _ => self.evaluate_stylex(modules, call),
            },
            Expression::TSAsExpression(inner) => self.evaluate(modules, &inner.expression),
            Expression::TSSatisfiesExpression(inner) => self.evaluate(modules, &inner.expression),
            Expression::ParenthesizedExpression(inner) => self.evaluate(modules, &inner.expression),
            _ => None,
        }
    }
}

/// `value` as JavaScript turns it into a string
fn js_string(value: &Constant) -> Option<String> {
    match value {
        Constant::String(text) => Some(text.clone()),
        Constant::Number(number) => Some(crate::utils::js_number_string(*number)),
        _ => None,
    }
}

fn member_of(object: &Constant, key: &str) -> Option<Constant> {
    match object {
        Constant::Object(object) => object.get(key).cloned(),
        Constant::Vars(vars) => vars
            .get(key)
            .map(|variable| Constant::String(format!("var({variable})"))),
        _ => None,
    }
}

fn fold_template(
    template: &oxc_ast::ast::TemplateLiteral<'_>,
    values: &[Constant],
) -> Option<Constant> {
    let mut text = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        text.push_str(quasi.value.cooked.as_deref()?);
        if let Some(value) = values.get(index) {
            text.push_str(&js_string(value)?);
        }
    }
    Some(Constant::String(text))
}

fn math_constant(name: &str) -> Option<Constant> {
    use std::f64::consts;
    let value = match name {
        "PI" => consts::PI,
        "E" => consts::E,
        "LN2" => consts::LN_2,
        "LN10" => consts::LN_10,
        "LOG2E" => consts::LOG2_E,
        "LOG10E" => consts::LOG10_E,
        "SQRT2" => consts::SQRT_2,
        "SQRT1_2" => consts::FRAC_1_SQRT_2,
        _ => return None,
    };
    Some(Constant::Number(value))
}

/// `Math.{name}(...arguments)`, folded only where every engine computes the
/// same result, so the CSS never depends on the platform that builds it
fn fold_math(name: &str, arguments: &[Constant]) -> Option<Constant> {
    let mut numbers = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let Constant::Number(number) = argument else {
            return None;
        };
        numbers.push(*number);
    }
    let first = numbers.first().copied();
    let value = match name {
        "abs" => first?.abs(),
        "ceil" => first?.ceil(),
        "floor" => first?.floor(),
        "trunc" => first?.trunc(),
        "sqrt" => first?.sqrt(),
        "sign" => {
            let x = first?;
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                x
            }
        }
        // JavaScript rounds a half up, toward +Infinity, where Rust rounds it
        // away from zero; `x - floor(x)` is exact for every double
        "round" => {
            let x = first?;
            let floor = x.floor();
            if x - floor >= 0.5 { floor + 1.0 } else { floor }
        }
        "max" => numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "min" => numbers.iter().copied().fold(f64::INFINITY, f64::min),
        "pow" => exact_power(first?, *numbers.get(1)?)?,
        _ => return None,
    };
    value.is_finite().then_some(Constant::Number(value))
}

/// An integer raised to a whole power, when the result is an exact integer
fn exact_power(base: f64, exponent: f64) -> Option<f64> {
    const EXACT: f64 = 9_007_199_254_740_992.0;
    if base.fract() != 0.0 || exponent.fract() != 0.0 || !(0.0..=64.0).contains(&exponent) {
        return None;
    }
    let mut result = 1.0_f64;
    for _ in 0..exponent as u8 {
        result *= base;
        if result.abs() > EXACT {
            return None;
        }
    }
    Some(result)
}

fn fold_binary(operator: BinaryOperator, left: &Constant, right: &Constant) -> Option<Constant> {
    let number = match (operator, left, right) {
        (BinaryOperator::Addition, Constant::Number(a), Constant::Number(b)) => a + b,
        (BinaryOperator::Subtraction, Constant::Number(a), Constant::Number(b)) => a - b,
        (BinaryOperator::Multiplication, Constant::Number(a), Constant::Number(b)) => a * b,
        (BinaryOperator::Division, Constant::Number(a), Constant::Number(b)) => a / b,
        (BinaryOperator::Remainder, Constant::Number(a), Constant::Number(b)) => a % b,
        (BinaryOperator::Addition, Constant::String(_), _)
        | (BinaryOperator::Addition, _, Constant::String(_)) => {
            return Some(Constant::String(js_string(left)? + &js_string(right)?));
        }
        _ => return None,
    };
    number.is_finite().then_some(Constant::Number(number))
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
            Expression::StaticMemberExpression(member) if self.is_global_math(&member.object) => {
                math_constant(member.property.name.as_str())
            }
            Expression::StaticMemberExpression(member) => member_of(
                &self.constant(&member.object)?,
                member.property.name.as_str(),
            ),
            Expression::CallExpression(call) => {
                let Expression::StaticMemberExpression(callee) = &call.callee else {
                    return None;
                };
                if !self.is_global_math(&callee.object) {
                    return None;
                }
                let arguments: Option<Vec<Constant>> = call
                    .arguments
                    .iter()
                    .map(|argument| self.operand(argument.as_expression()?))
                    .collect();
                fold_math(callee.property.name.as_str(), &arguments?)
            }
            Expression::ComputedMemberExpression(member) => {
                let key = js_string(&self.operand(&member.expression)?)?;
                member_of(&self.constant(&member.object)?, &key)
            }
            // Folded only when they read a constant, leaving other code as written
            Expression::TemplateLiteral(template)
                if template
                    .expressions
                    .iter()
                    .any(|e| self.constant(e).is_some()) =>
            {
                let values: Option<Vec<Constant>> = template
                    .expressions
                    .iter()
                    .map(|expression| self.operand(expression))
                    .collect();
                fold_template(template, &values?)
            }
            Expression::BinaryExpression(binary)
                if self.constant(&binary.left).is_some()
                    || self.constant(&binary.right).is_some() =>
            {
                fold_binary(
                    binary.operator,
                    &self.operand(&binary.left)?,
                    &self.operand(&binary.right)?,
                )
            }
            Expression::UnaryExpression(unary)
                if unary.operator == oxc_syntax::operator::UnaryOperator::UnaryNegation =>
            {
                match self.constant(&unary.argument)? {
                    Constant::Number(number) => Some(Constant::Number(-number)),
                    _ => None,
                }
            }
            Expression::ParenthesizedExpression(inner) => self.constant(&inner.expression),
            _ => None,
        }
    }

    /// A constant read or a literal
    fn operand(&self, expression: &Expression<'a>) -> Option<Constant> {
        self.constant(expression).or_else(|| match expression {
            Expression::StringLiteral(literal) => Some(Constant::String(literal.value.to_string())),
            _ => crate::utils::js_number_literal(expression).map(Constant::Number),
        })
    }

    fn is_global_math(&self, expression: &Expression<'a>) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == "Math"
                && identifier
                    .reference_id
                    .get()
                    .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                    .is_none())
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
            _ => None,
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
