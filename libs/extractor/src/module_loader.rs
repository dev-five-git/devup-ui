use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt::Write;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, Declaration, ExportDefaultDeclarationKind, ImportDeclarationSpecifier,
    ModuleExportName, Statement,
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{ExtractOption, ModuleResolver, utils::is_vanilla_extract_file};

/// The object the package's API is bound to while a stylesheet runs
pub(crate) const PACKAGE_BINDING: &str = "__vanilla_extract__";

/// The end of the error a module read before its evaluation throws
pub(crate) const IMPORT_CYCLE: &str = "before its initialization: it is part of an import cycle";

const MODULE_HELPER: &str = "function __module__(path) { let started = false; const module = new Proxy({}, { get(target, key, receiver) { if (!started && typeof key === \"string\") throw new ReferenceError(`Cannot access '${key}' of '${path}' before its initialization: it is part of an import cycle`); return Reflect.get(target, key, receiver); } }); return { module, start() { started = true; } }; }\n";

thread_local! {
    /// Stylesheets being evaluated, outermost first: a stylesheet importing one
    /// that is still being evaluated is a cycle no evaluation order can satisfy
    static EVALUATING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Marks `filename` as being evaluated until dropped
pub(crate) struct Evaluating;

impl Evaluating {
    pub(crate) fn enter(filename: &str) -> Self {
        EVALUATING.with_borrow_mut(|stack| stack.push(filename.to_string()));
        Self
    }
}

impl Drop for Evaluating {
    fn drop(&mut self) {
        EVALUATING.with_borrow_mut(Vec::pop);
    }
}

/// Whether a stylesheet is being evaluated, so the one extracted now is loaded
/// by it and must not fall back to plain extraction
pub(crate) fn loading_for_stylesheet() -> bool {
    EVALUATING.with_borrow(|stack| !stack.is_empty())
}

fn is_evaluating(filename: &str) -> bool {
    EVALUATING.with_borrow(|stack| stack.iter().any(|entry| entry == filename))
}

/// Loads what a stylesheet imports, each module once, as JavaScript that
/// defines the module's exports object ahead of the stylesheet
pub(crate) struct ModuleLoader<'r> {
    resolver: Option<&'r ModuleResolver>,
    option: &'r ExtractOption,
    /// Definitions of the loaded modules, each after the ones it uses
    definitions: Vec<String>,
    loaded: FxHashMap<String, String>,
    /// `(path, name)` of the modules being defined, outermost first
    loading: Vec<(String, String)>,
    /// Names of modules imported before they finished evaluating, whose
    /// bindings are read when used, as ES modules read an import cycle
    pending: FxHashSet<String>,
    next_module: usize,
    /// Every file read, including those the loaded stylesheets read
    pub dependencies: BTreeSet<String>,
    /// What the stylesheet imports for its side effects, and the stylesheets it
    /// imports, which emit their own styles: its output keeps importing them
    pub kept_imports: Vec<String>,
    /// Stand in for what cannot be loaded, and let a module that throws only
    /// lose its own bindings
    lenient: bool,
}

impl<'r> ModuleLoader<'r> {
    pub(crate) fn new(resolver: Option<&'r ModuleResolver>, option: &'r ExtractOption) -> Self {
        Self {
            resolver,
            option,
            definitions: Vec::new(),
            loaded: FxHashMap::default(),
            loading: Vec::new(),
            pending: FxHashSet::default(),
            next_module: 0,
            dependencies: BTreeSet::new(),
            kept_imports: Vec::new(),
            lenient: false,
        }
    }

    pub(crate) const fn lenient(mut self) -> Self {
        self.lenient = true;
        self
    }

    /// The name of the exports object of `specifier`: in lenient mode the
    /// style packages, and modules that cannot be loaded, are the stand-in
    /// `PACKAGE_BINDING` holds
    fn module(&mut self, specifier: &str, importer: &str, direct: bool) -> Result<String, String> {
        if !self.lenient {
            return self.load(specifier, importer, direct);
        }
        if specifier == crate::STYLEX_PACKAGE || self.option.import_aliases.contains_key(specifier)
        {
            return Ok(PACKAGE_BINDING.to_string());
        }
        Ok(self
            .load(specifier, importer, direct)
            .unwrap_or_else(|_| PACKAGE_BINDING.to_string()))
    }

    fn keep_import(&mut self, specifier: &str) {
        if !self.kept_imports.iter().any(|kept| kept == specifier) {
            self.kept_imports.push(specifier.to_string());
        }
    }

    /// The loaded modules' definitions, to run before the stylesheet
    pub(crate) fn prelude(&self) -> String {
        self.definitions.concat()
    }

    /// The name of the exports object of `specifier` imported by `importer`
    fn load(&mut self, specifier: &str, importer: &str, direct: bool) -> Result<String, String> {
        let resolver = self
            .resolver
            .ok_or_else(|| format!("Cannot load '{specifier}' without a module resolver"))?;
        let module = resolver(specifier, importer)
            .ok_or_else(|| format!("Cannot resolve '{specifier}' from '{importer}'"))?;
        let stylesheet = is_vanilla_extract_file(&module.path);
        if direct && stylesheet {
            self.keep_import(specifier);
        }
        if let Some(name) = self.loaded.get(&module.path) {
            return Ok(name.clone());
        }
        if let Some((_, name)) = self.loading.iter().find(|(path, _)| *path == module.path) {
            let name = name.clone();
            self.pending.insert(name.clone());
            return Ok(name);
        }
        let name = format!("__module_{}__", self.next_module);
        self.next_module += 1;
        // Created before the modules it imports, so a cycle among them can
        // reach it; reading it before it starts evaluating is an error
        if self.definitions.is_empty() {
            self.definitions.push(MODULE_HELPER.to_string());
        }
        self.definitions.push(format!(
            "const {name}$ = __module__({:?});\nconst {name} = {name}$.module;\n",
            module.path
        ));
        if is_evaluating(&module.path) {
            // The stylesheet importing it is evaluated on its own, so it never
            // starts here
            self.pending.insert(name.clone());
            self.loaded.insert(module.path, name.clone());
            return Ok(name);
        }
        self.dependencies.insert(module.path.clone());
        self.loading.push((module.path.clone(), name.clone()));
        let defined = self.define(&name, module, stylesheet, resolver);
        let (path, _) = self.loading.pop().unwrap_or_default();
        defined?;
        self.loaded.insert(path, name.clone());
        Ok(name)
    }

    fn define(
        &mut self,
        name: &str,
        module: crate::ResolvedModule,
        stylesheet: bool,
        resolver: &ModuleResolver,
    ) -> Result<(), String> {
        let code = if stylesheet {
            // Extracted the way the bundler extracts it, so the names it
            // exports are the ones its own CSS uses
            let output = crate::extract_with_source_map(
                &module.path,
                &module.code,
                self.option.clone(),
                false,
                Some(resolver),
            )
            .map_err(|error| error.to_string())?;
            self.dependencies.extend(output.dependencies);
            output.code
        } else {
            module.code
        };
        let script = crate::vanilla_extract::strip_typescript(&code, &module.path);
        let module_script = module_script(&script, &module.path, self, false)?;
        // Live bindings: a read before the binding is initialized fails as it
        // does in an ES module
        let getters: Vec<String> = module_script
            .exports
            .iter()
            .map(|(exported, local)| {
                format!("{exported:?}: {{ get() {{ return {local}; }}, enumerable: true }}")
            })
            .collect();
        let (open, close) = if self.lenient {
            ("try {\n", "} catch {}\n")
        } else {
            ("", "")
        };
        if module_script.commonjs {
            // Its exports are what `module.exports` holds once it ran; the
            // default follows bundler interop (`__esModule` marks a compiled
            // ES module)
            self.definitions.push(format!(
                "{open}(function () {{\n{name}$.start();\nconst module = {{ exports: {{}} }};\nconst exports = module.exports;\n{}\nconst e = module.exports;\nObject.defineProperty({name}, \"__exports__\", {{ value: e }});\nif (e !== null && (typeof e === \"object\" || typeof e === \"function\")) for (const key of Object.keys(e)) if (key !== \"default\") Object.defineProperty({name}, key, {{ get: () => e[key], enumerable: true }});\nObject.defineProperty({name}, \"default\", {{ value: e !== null && typeof e === \"object\" && e.__esModule ? e.default : e, enumerable: true }});\n}})();\n{close}",
                module_script.body,
            ));
            return Ok(());
        }
        let mut spreads = String::new();
        for spread in &module_script.spreads {
            let _ = writeln!(
                spreads,
                "for (const key of Object.keys({spread})) if (key !== \"default\" && !(key in {name})) Object.defineProperty({name}, key, {{ get: () => {spread}[key], enumerable: true }});"
            );
        }
        self.definitions.push(format!(
            "{open}(function () {{\n{name}$.start();\nObject.defineProperties({name}, {{ {} }});\n{spreads}{}\n}})();\n{close}",
            getters.join(", "),
            module_script.body,
        ));
        Ok(())
    }
}

/// A module turned into a script: imports bound to the loaded modules, export
/// keywords dropped
pub(crate) struct ModuleScript {
    pub body: String,
    /// `(exported name, expression)`
    exports: Vec<(String, String)>,
    /// Modules re-exported whole
    spreads: Vec<String>,
    /// Written with `module.exports` rather than `export`
    commonjs: bool,
}

pub(crate) fn module_script(
    script: &str,
    filename: &str,
    loader: &mut ModuleLoader<'_>,
    entry: bool,
) -> Result<ModuleScript, String> {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, script, SourceType::mjs())
        .parse()
        .program;
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    let package = loader.option.package.clone();

    // Imports of a module still evaluating, or in lenient mode of one that may
    // have thrown, are read where they are used, as ES modules read an import
    // cycle
    let mut modules: FxHashMap<u32, String> = FxHashMap::default();
    let mut lazy: FxHashMap<SymbolId, String> = FxHashMap::default();
    let mut lazy_names: FxHashMap<String, String> = FxHashMap::default();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let source = import.source.value.as_str();
        let specifiers = import.specifiers.as_deref().map_or(&[][..], |s| s);
        if specifiers.is_empty() {
            if entry && !source.starts_with(package.as_str()) {
                loader.keep_import(source);
            }
            continue;
        }
        let module = if source == package {
            PACKAGE_BINDING.to_string()
        } else {
            loader.module(source, filename, entry)?
        };
        if loader.lenient || loader.pending.contains(&module) {
            let lazy_module = &module;
            for specifier in specifiers {
                let binding = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        format!("{lazy_module}[{:?}]", specifier.imported.name())
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        format!("{lazy_module}[\"default\"]")
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => module.clone(),
                };
                let local = specifier.local();
                if let Some(symbol) = local.symbol_id.get() {
                    lazy.insert(symbol, binding.clone());
                }
                lazy_names.insert(local.name.to_string(), binding);
            }
        }
        modules.insert(import.span.start, module);
    }
    let mut replacements: Vec<(u32, u32, String)> = Vec::new();
    for (symbol, binding) in &lazy {
        for reference in semantic.scoping().get_resolved_reference_ids(*symbol) {
            let node = semantic.scoping().get_reference(*reference).node_id();
            let span = semantic.nodes().kind(node).span();
            let replacement = match semantic.nodes().parent_kind(node) {
                AstKind::ObjectProperty(property) if property.shorthand => {
                    format!(
                        "{}: {binding}",
                        &script[span.start as usize..span.end as usize]
                    )
                }
                AstKind::ExportSpecifier(_) => continue,
                _ => binding.clone(),
            };
            replacements.push((span.start, span.end, replacement));
        }
    }
    // CommonJS: equire of a literal path loads the module like an import
    let commonjs = !program.body.iter().any(Statement::is_module_declaration)
        && semantic
            .scoping()
            .root_unresolved_references()
            .keys()
            .any(|name| matches!(name.as_str(), "module" | "exports" | "require"));
    if commonjs
        && let Some(references) = semantic
            .scoping()
            .root_unresolved_references()
            .get("require")
    {
        for reference in references {
            let node = semantic.scoping().get_reference(*reference).node_id();
            if let AstKind::CallExpression(call) = semantic.nodes().parent_kind(node)
                && let [Argument::StringLiteral(specifier)] = call.arguments.as_slice()
            {
                let module = loader.module(specifier.value.as_str(), filename, entry)?;
                replacements.push((
                    call.span.start,
                    call.span.end,
                    format!("(\"__exports__\" in {module} ? {module}.__exports__ : {module})"),
                ));
            }
        }
    }
    replacements.sort_by_key(|(start, ..)| *start);
    let text = |span: oxc_span::Span| {
        let mut code = String::new();
        let mut copied = span.start;
        for (start, end, replacement) in &replacements {
            if *start >= span.start && *end <= span.end {
                code.push_str(&script[copied as usize..*start as usize]);
                code.push_str(replacement);
                copied = *end;
            }
        }
        code.push_str(&script[copied as usize..span.end as usize]);
        code
    };

    let mut body = String::with_capacity(script.len());
    let mut exports = Vec::new();
    let mut spreads = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(import) => {
                let Some(module) = modules.get(&import.span.start) else {
                    continue;
                };
                if loader.lenient || loader.pending.contains(module) {
                    continue;
                }
                let mut named = Vec::new();
                for specifier in import.specifiers.iter().flatten() {
                    match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(specifier) => named.push(
                            format!("{:?}: {}", specifier.imported.name(), specifier.local.name),
                        ),
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                            named.push(format!("\"default\": {}", specifier.local.name));
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                            let _ = writeln!(body, "const {} = {module};", specifier.local.name);
                        }
                    }
                }
                if !named.is_empty() {
                    let _ = writeln!(body, "const {{ {} }} = {module};", named.join(", "));
                }
            }
            Statement::ExportDeclaration(export) => {
                body.push_str(&text(export.declaration.span()));
                body.push('\n');
                for name in declared_names(&export.declaration) {
                    exports.push((name.clone(), name));
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    let local = export_name(&specifier.local);
                    exports.push((
                        export_name(&specifier.exported),
                        lazy_names.get(&local).cloned().unwrap_or(local),
                    ));
                }
            }
            Statement::ExportFromDeclaration(export) => {
                let module = loader.module(export.source.value.as_str(), filename, entry)?;
                for specifier in &export.specifiers {
                    exports.push((
                        export_name(&specifier.exported),
                        format!("{module}[{:?}]", export_name(&specifier.local)),
                    ));
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let id = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        function.id.as_ref()
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => class.id.as_ref(),
                    _ => None,
                };
                let declaration = text(export.declaration.span());
                let local = if let Some(id) = id {
                    body.push_str(&declaration);
                    id.name.to_string()
                } else {
                    let _ = write!(body, "const __default__ = ({declaration});");
                    "__default__".to_string()
                };
                body.push('\n');
                exports.push(("default".to_string(), local));
            }
            Statement::ExportAllDeclaration(export) => {
                let module = loader.module(export.source.value.as_str(), filename, entry)?;
                match &export.exported {
                    Some(exported) => exports.push((export_name(exported), module)),
                    None => spreads.push(module),
                }
            }
            statement => {
                body.push_str(&text(statement.span()));
                body.push('\n');
            }
        }
    }
    Ok(ModuleScript {
        body,
        exports,
        spreads,
        commonjs,
    })
}
fn export_name(name: &ModuleExportName<'_>) -> String {
    name.name().to_string()
}

fn declared_names(declaration: &Declaration<'_>) -> Vec<String> {
    match declaration {
        Declaration::VariableDeclaration(declaration) => declaration
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .map(|identifier| identifier.name.to_string())
            .collect(),
        declaration => declaration
            .id()
            .iter()
            .map(|id| id.name.to_string())
            .collect(),
    }
}
