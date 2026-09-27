use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt::Write;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, ImportDeclarationSpecifier, ModuleExportName,
    Statement,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use rustc_hash::FxHashMap;

use crate::{ExtractOption, ModuleResolver, utils::is_vanilla_extract_file};

/// The object the package's API is bound to while a stylesheet runs
pub(crate) const PACKAGE_BINDING: &str = "__vanilla_extract__";

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
    loading: Vec<String>,
    /// Every file read, including those the loaded stylesheets read
    pub dependencies: BTreeSet<String>,
    /// What the stylesheet imports for its side effects, and the stylesheets it
    /// imports, which emit their own styles: its output keeps importing them
    pub kept_imports: Vec<String>,
}

impl<'r> ModuleLoader<'r> {
    pub(crate) fn new(resolver: Option<&'r ModuleResolver>, option: &'r ExtractOption) -> Self {
        Self {
            resolver,
            option,
            definitions: Vec::new(),
            loaded: FxHashMap::default(),
            loading: Vec::new(),
            dependencies: BTreeSet::new(),
            kept_imports: Vec::new(),
        }
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
        // A stylesheet falling back to plain extraction still imports the other
        // side of the cycle, so the cycle has to end here
        if self.loading.contains(&module.path) || is_evaluating(&module.path) {
            return Err(format!("Circular import of '{}'", module.path));
        }
        self.dependencies.insert(module.path.clone());
        self.loading.push(module.path.clone());
        let name = self.define(module, stylesheet, resolver);
        self.loading.pop();
        let (path, name) = name?;
        self.loaded.insert(path, name.clone());
        Ok(name)
    }

    fn define(
        &mut self,
        module: crate::ResolvedModule,
        stylesheet: bool,
        resolver: &ModuleResolver,
    ) -> Result<(String, String), String> {
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
        let script = crate::vanilla_extract::strip_typescript(&code);
        let module_script = module_script(&script, &module.path, self, false)?;
        let name = format!("__module_{}__", self.definitions.len());
        let exports: Vec<String> = module_script
            .spreads
            .iter()
            .map(|spread| format!("...{spread}"))
            .chain(
                module_script
                    .exports
                    .iter()
                    .map(|(exported, local)| format!("{exported:?}: {local}")),
            )
            .collect();
        self.definitions.push(format!(
            "const {name} = (function () {{\n{}\nreturn {{ {} }};\n}})();\n",
            module_script.body,
            exports.join(", ")
        ));
        Ok((module.path, name))
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
    let package = loader.option.package.as_str();
    let text = |span: oxc_span::Span| &script[span.start as usize..span.end as usize];
    let mut body = String::with_capacity(script.len());
    let mut exports = Vec::new();
    let mut spreads = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(import) => {
                let source = import.source.value.as_str();
                let specifiers = import.specifiers.as_deref().map_or(&[][..], |s| s);
                if specifiers.is_empty() {
                    if entry && !source.starts_with(package) {
                        loader.keep_import(source);
                    }
                    continue;
                }
                let module = if source == package {
                    PACKAGE_BINDING.to_string()
                } else {
                    loader.load(source, filename, entry)?
                };
                let mut named = Vec::new();
                for specifier in specifiers {
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
                body.push_str(text(export.declaration.span()));
                body.push('\n');
                for name in declared_names(&export.declaration) {
                    exports.push((name.clone(), name));
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    exports.push((
                        export_name(&specifier.exported),
                        export_name(&specifier.local),
                    ));
                }
            }
            Statement::ExportFromDeclaration(export) => {
                let module = loader.load(export.source.value.as_str(), filename, entry)?;
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
                    body.push_str(declaration);
                    id.name.to_string()
                } else {
                    let _ = write!(body, "const __default__ = ({declaration});");
                    "__default__".to_string()
                };
                body.push('\n');
                exports.push(("default".to_string(), local));
            }
            Statement::ExportAllDeclaration(export) => {
                let module = loader.load(export.source.value.as_str(), filename, entry)?;
                match &export.exported {
                    Some(exported) => exports.push((export_name(exported), module)),
                    None => spreads.push(module),
                }
            }
            statement => {
                body.push_str(text(statement.span()));
                body.push('\n');
            }
        }
    }
    Ok(ModuleScript {
        body,
        exports,
        spreads,
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
