use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::rc::Rc;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, Declaration, ExportDefaultDeclarationKind, ImportDeclarationSpecifier,
    ModuleExportName, Statement,
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{ExtractOption, ModuleResolver, utils::is_vanilla_extract_file};

mod import_bindings;
pub(crate) mod operations;
mod retained_css;
mod script;
#[cfg(test)]
mod tests;
mod validate;

use import_bindings::{BindingSource, ImportBindings};
use retained_css::{CssImport, RetainedCss};
pub(crate) use script::{Body, Origin, SCRIPT_PATH, Script, Unit};
pub(crate) use validate::validate;

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

/// Whether the stylesheet evaluated now is one another evaluation imports
pub(crate) fn evaluating_import() -> bool {
    EVALUATING.with_borrow(|stack| stack.len() > 1)
}

fn is_evaluating(filename: &str) -> bool {
    EVALUATING.with_borrow(|stack| stack.iter().any(|entry| entry == filename))
}

/// Why a module could not be loaded
enum LoadError {
    NoResolver,
    Unresolved,
    CssPath(retained_css::PathError),
    /// The module's own error, told where it is already
    Failed(String),
}

impl LoadError {
    /// The error told for the import of `specifier` by `importer` at the place
    /// `place` gives
    fn describe(self, specifier: &str, importer: &str, place: impl FnOnce() -> String) -> String {
        match self {
            Self::NoResolver | Self::Unresolved if retained_css::is_css(specifier) => format!(
                "{}: Cannot resolve CSS '{specifier}' from '{importer}': the stylesheet loads CSS through this helper and the file cannot be found. Fix: correct the path or import CSS from a component module; after creating a missing file, re-save the stylesheet to retry",
                place()
            ),
            Self::NoResolver => format!(
                "{}: Cannot load '{specifier}' without a module resolver. Fix: build through a Devup UI plugin, which resolves imports, or write what '{specifier}' provides in this file",
                place()
            ),
            Self::Unresolved => format!(
                "{}: Cannot resolve '{specifier}' from '{importer}'. Fix: check that '{specifier}' exists with a supported extension and that the bundler's resolver can find it from this file",
                place()
            ),
            Self::Failed(error) => error,
            Self::CssPath(error) => format!("{}: {}", place(), error.describe()),
        }
    }
}

/// A module's definition, as a part of the script
enum Definition {
    Generated(String),
    Module {
        head: String,
        body: String,
        tail: String,
        origin: Rc<Origin>,
    },
}

/// Loads what a stylesheet imports, each module once, as JavaScript that
/// defines the module's exports object ahead of the stylesheet
pub(crate) struct ModuleLoader<'r> {
    resolver: Option<&'r ModuleResolver>,
    option: &'r ExtractOption,
    /// Definitions of the loaded modules, each after the ones it uses
    definitions: Vec<Definition>,
    loaded: FxHashMap<String, String>,
    /// `(path, name)` of the modules being defined, outermost first
    loading: Vec<(String, String)>,
    /// Names of modules imported before they finished evaluating, whose
    /// bindings are read when used, as ES modules read an import cycle
    pending: FxHashSet<String>,
    next_module: usize,
    retained_css: RetainedCss,
    css_bindings: FxHashMap<String, String>,
    /// Every file read, including those the loaded stylesheets read
    pub dependencies: BTreeSet<String>,
    /// What the stylesheet imports for its side effects, and the stylesheets it
    /// imports, which emit their own styles: its output keeps importing them
    pub kept_imports: Vec<String>,
    pub imported_atoms: crate::vanilla_extract::producer_atoms::ProducerAtoms,
    pub imported_references: crate::vanilla_extract::style_references::StyleReferences,
}

impl<'r> ModuleLoader<'r> {
    /// Instantiates CSS objects without inventing any of their exports.
    pub(crate) fn prepare_css(
        &self,
        context: &mut boa_engine::Context,
    ) -> boa_engine::JsResult<()> {
        for (name, file) in &self.css_bindings {
            let object = crate::evaluation_sandbox::css_object(context, file)?;
            context.register_global_property(
                boa_engine::JsString::from(name.as_str()),
                object,
                boa_engine::property::Attribute::empty(),
            )?;
        }
        Ok(())
    }

    pub(crate) fn new(resolver: Option<&'r ModuleResolver>, option: &'r ExtractOption) -> Self {
        Self {
            resolver,
            option,
            definitions: Vec::new(),
            loaded: FxHashMap::default(),
            loading: Vec::new(),
            pending: FxHashSet::default(),
            next_module: 0,
            retained_css: RetainedCss::default(),
            css_bindings: FxHashMap::default(),
            dependencies: BTreeSet::new(),
            kept_imports: Vec::new(),
            imported_atoms: Default::default(),
            imported_references: Default::default(),
        }
    }

    fn keep_import(&mut self, specifier: &str) {
        if !self.kept_imports.iter().any(|kept| kept == specifier) {
            self.kept_imports.push(specifier.to_string());
        }
    }

    fn keep_entry_import(&mut self, specifier: &str) {
        if retained_css::is_generated(specifier, self.option)
            || !retained_css::is_css(specifier)
            || self.retained_css.keep_root(specifier, self.resolver)
        {
            self.keep_import(specifier);
        }
    }

    /// The script to run: the loaded modules' definitions, then the `entry`
    /// stylesheet
    pub(crate) fn script(&self, entry: &ModuleScript) -> Script {
        let mut script = Script::default();
        for definition in &self.definitions {
            match definition {
                Definition::Generated(text) => script.generated(text),
                Definition::Module {
                    head,
                    body,
                    tail,
                    origin,
                } => {
                    script.generated(head);
                    script.body(body, origin);
                    script.generated(tail);
                }
            }
        }
        script.body(&entry.body, &entry.origin);
        script
    }

    /// The name of the exports object of `specifier` imported by `importer`
    fn load(&mut self, specifier: &str, importer: &str, direct: bool) -> Result<String, LoadError> {
        let resolver = self.resolver.ok_or(LoadError::NoResolver)?;
        let module = resolver(specifier, importer).ok_or(LoadError::Unresolved)?;
        if retained_css::is_css(&module.path) {
            if let Some(kept) = self
                .retained_css
                .retain(
                    CssImport {
                        specifier,
                        path: &module.path,
                        direct,
                    },
                    resolver,
                    &mut self.kept_imports,
                )
                .map_err(LoadError::CssPath)?
            {
                self.keep_import(&kept);
            }
            if let Some(name) = self.loaded.get(&module.path) {
                return Ok(name.clone());
            }
            let name = format!("__css_{}__", self.next_module);
            self.next_module += 1;
            self.css_bindings.insert(name.clone(), module.path.clone());
            self.pending.insert(name.clone());
            self.loaded.insert(module.path, name.clone());
            return Ok(name);
        }
        let stylesheet = is_vanilla_extract_file(&module.path)
            || (self
                .option
                .import_aliases
                .contains_key("@vanilla-extract/css")
                && crate::ordinary_ve::is_module(&module.path, &module.code));
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
            self.definitions
                .push(Definition::Generated(MODULE_HELPER.to_string()));
        }
        self.definitions.push(Definition::Generated(format!(
            "const {name}$ = __module__({:?});\nconst {name} = {name}$.module;\n",
            module.path
        )));
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
        defined.map_err(LoadError::Failed)?;
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
        validate(crate::vanilla_extract::Stylesheet {
            filename: &module.path,
            code: &module.code,
            source: &module.code,
            edits: &[],
        })?;
        let unit = if stylesheet {
            // Extracted the way the bundler extracts it, so the names it
            // exports are the ones its own CSS uses
            let result = crate::extract_source(
                &module.path,
                &module.code,
                None,
                false,
                self.option.clone(),
                true,
                Some(resolver),
            )
            .map_err(|error| error.to_string())?;
            let output = result.output;
            let unit = Unit::retained(&module.path, &output, &module.code)?;
            self.dependencies.extend(output.dependencies);
            self.imported_atoms.merge(result.atoms);
            self.imported_references.merge(result.references);
            unit
        } else {
            Unit::written(&module.path, &module.code, &module.code, &[])?
        };
        let module_script = module_script(&unit, self, false)?;
        // Live bindings: a read before the binding is initialized fails as it
        // does in an ES module
        let getters: Vec<String> = module_script
            .exports
            .iter()
            .map(|(exported, local)| {
                format!("{exported:?}: {{ get() {{ return {local}; }}, enumerable: true }}")
            })
            .collect();
        if module_script.commonjs {
            // Its exports are what `module.exports` holds once it ran; the
            // default follows bundler interop (`__esModule` marks a compiled
            // ES module)
            self.definitions.push(Definition::Module {
                head: format!(
                    "(function () {{\n{name}$.start();\nconst module = {{ exports: {{}} }};\nconst exports = module.exports;\n"
                ),
                body: module_script.body,
                tail: format!(
                    "\nconst e = module.exports;\nObject.defineProperty({name}, \"__exports__\", {{ value: e }});\nif (e !== null && (typeof e === \"object\" || typeof e === \"function\")) for (const key of Object.keys(e)) if (key !== \"default\") Object.defineProperty({name}, key, {{ get: () => e[key], enumerable: true }});\nObject.defineProperty({name}, \"default\", {{ value: e !== null && typeof e === \"object\" && e.__esModule ? e.default : e, enumerable: true }});\n}})();\n"
                ),
                origin: module_script.origin,
            });
            return Ok(());
        }
        let mut spreads = String::new();
        for spread in &module_script.spreads {
            let _ = writeln!(
                spreads,
                "for (const key of Object.keys({spread})) if (key !== \"default\" && !(key in {name})) Object.defineProperty({name}, key, {{ get: () => {spread}[key], enumerable: true }});"
            );
        }
        self.definitions.push(Definition::Module {
            head: format!(
                "(function () {{\n{name}$.start();\nObject.defineProperties({name}, {{ {} }});\n{spreads}",
                getters.join(", ")
            ),
            body: module_script.body,
            tail: "\n})();\n".to_string(),
            origin: module_script.origin,
        });
        Ok(())
    }
}

/// A module turned into a script: imports bound to the loaded modules, export
/// keywords dropped
pub(crate) struct ModuleScript {
    pub body: String,
    /// What `body` was made of, to tell where its code was written
    pub origin: Rc<Origin>,
    /// `(exported name, expression)`
    pub(crate) exports: Vec<(String, String)>,
    /// Modules re-exported whole
    pub(crate) spreads: Vec<String>,
    /// Written with `module.exports` rather than `export`
    commonjs: bool,
}

/// The replacements of a module's code, applied as its statements are copied
struct Rewrites<'s> {
    script: &'s str,
    /// `(start, end, replacement)`, by start
    list: Vec<(u32, u32, String)>,
}

impl Rewrites<'_> {
    /// Copies the text of `span` into `body`, replacements made
    fn write(&self, body: &mut Body, span: Span) {
        let mut copied = span.start as usize;
        for (start, end, replacement) in &self.list {
            if *start >= span.start && *end <= span.end {
                body.copy(self.script, copied, *start as usize);
                body.synthesize(*start as usize, replacement);
                copied = *end as usize;
            }
        }
        body.copy(self.script, copied, span.end as usize);
    }
}

pub(crate) fn module_script(
    unit: &Rc<Unit>,
    loader: &mut ModuleLoader<'_>,
    entry: bool,
) -> Result<ModuleScript, String> {
    let (script, filename) = (unit.script(), unit.filename());
    if entry {
        loader.retained_css.root = filename.to_string();
    }
    let load_module = |loader: &mut ModuleLoader<'_>, specifier: &str, at: u32| {
        loader
            .load(specifier, filename, entry)
            .map_err(|error| error.describe(specifier, filename, || unit.place(at as usize)))
    };
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, script, SourceType::mjs()).parse();
    let program = parsed.program;
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .build(&program);
    if let Some(error) = parsed
        .diagnostics
        .iter()
        .chain(built.diagnostics.iter())
        .next()
    {
        return Err(format!(
            "{}: JS execution error: SyntaxError: {error}. Fix: correct the syntax at this location",
            unit.place(error.labels.first().map_or(0, |label| {
                usize::try_from(label.offset()).unwrap_or(script.len())
            }))
        ));
    }
    let semantic = built.semantic;
    let package = loader.option.package.clone();

    // Imports of a module still evaluating are read where they are used, as ES
    // modules read an import cycle
    let mut modules: FxHashMap<u32, String> = FxHashMap::default();
    let mut bindings = ImportBindings::default();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let source = import.source.value.as_str();
        let specifiers = import.specifiers.as_deref().map_or(&[][..], |s| s);
        if specifiers.is_empty() {
            if !crate::package_specifier::is_package(source, &package)
                && (entry || !retained_css::is_generated(source, loader.option))
            {
                if entry {
                    loader.keep_entry_import(source);
                } else {
                    load_module(loader, source, import.source.span.start)?;
                }
            }
            continue;
        }
        let module = if source == package {
            PACKAGE_BINDING.to_string()
        } else {
            load_module(loader, source, import.source.span.start)?
        };
        if loader.pending.contains(&module) {
            let source = match loader.css_bindings.get(&module) {
                Some(_) => BindingSource::Css(&module),
                None => BindingSource::Module(&module),
            };
            bindings.link(specifiers, source);
        }
        modules.insert(import.span.start, module);
    }
    let mut replacements = bindings.rewrites(&semantic, script);
    // CommonJS: `require` of a literal path loads the module like an import
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
                let module = load_module(loader, specifier.value.as_str(), specifier.span.start)?;
                let replacement = if loader.css_bindings.contains_key(&module) {
                    module
                } else {
                    format!("(\"__exports__\" in {module} ? {module}.__exports__ : {module})")
                };
                replacements.push((call.span.start, call.span.end, replacement));
            }
        }
    }
    replacements.sort_by_key(|(start, ..)| *start);
    let rewrites = Rewrites {
        script,
        list: replacements,
    };

    let mut body = Body::new(script.len());
    let mut exports = Vec::new();
    let mut spreads = Vec::new();
    for statement in &program.body {
        let end = statement.span().end as usize;
        match statement {
            Statement::ImportDeclaration(import) => {
                let Some(module) = modules.get(&import.span.start) else {
                    continue;
                };
                if loader.pending.contains(module) {
                    continue;
                }
                let at = import.span.start as usize;
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
                            body.synthesize(
                                at,
                                &format!("const {} = {module};\n", specifier.local.name),
                            );
                        }
                    }
                }
                if !named.is_empty() {
                    body.synthesize(
                        at,
                        &format!("const {{ {} }} = {module};\n", named.join(", ")),
                    );
                }
            }
            Statement::ExportDeclaration(export) => {
                rewrites.write(&mut body, export.declaration.span());
                body.synthesize(end, "\n");
                for name in declared_names(&export.declaration) {
                    exports.push((name.clone(), name));
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    let local = export_name(&specifier.local);
                    exports.push((export_name(&specifier.exported), bindings.exported(local)));
                }
            }
            Statement::ExportFromDeclaration(export) => {
                let module = load_module(
                    loader,
                    export.source.value.as_str(),
                    export.source.span.start,
                )?;
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
                let declaration = export.declaration.span();
                let local = if let Some(id) = id {
                    rewrites.write(&mut body, declaration);
                    id.name.to_string()
                } else {
                    let default = crate::fresh_name::fresh_name("__default__", script);
                    body.synthesize(declaration.start as usize, &format!("const {default} = ("));
                    rewrites.write(&mut body, declaration);
                    body.synthesize(declaration.end as usize, ");");
                    default
                };
                body.synthesize(end, "\n");
                exports.push(("default".to_string(), local));
            }
            Statement::ExportAllDeclaration(export) => {
                let module = load_module(
                    loader,
                    export.source.value.as_str(),
                    export.source.span.start,
                )?;
                match &export.exported {
                    Some(exported) => exports.push((export_name(exported), module)),
                    None => spreads.push(module),
                }
            }
            statement => {
                rewrites.write(&mut body, statement.span());
                body.synthesize(end, "\n");
            }
        }
    }
    let (body, origin) = body.finish(unit);
    Ok(ModuleScript {
        body,
        origin,
        exports,
        spreads,
        commonjs,
    })
}
fn export_name(name: &ModuleExportName<'_>) -> String {
    name.name().to_string()
}

pub(crate) fn declared_names(declaration: &Declaration<'_>) -> Vec<String> {
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

#[cfg(test)]
mod coverage_tests;
