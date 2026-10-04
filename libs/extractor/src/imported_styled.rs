//! Styled components other modules define, read where a file extends one,
//! selects one in CSS text or gives one a `css` prop, so they compose exactly
//! as the ones the file defines itself.

use std::collections::BTreeSet;
use std::rc::Rc;

use oxc_allocator::{Allocator, GetAllocator};
use oxc_ast::ast::{
    CallExpression, Declaration, Expression, ImportDeclarationSpecifier, JSXAttributeItem,
    JSXAttributeName, JSXElementName, JSXOpeningElement, Program, Statement,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{Visit, VisitMut, walk};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::css_prop::CssProp;
use crate::extractor::extract_style_from_styled::StyledDefinition;
use crate::import_alias_visit::{Aliased, transform_import_aliases_with_edits};
use crate::utils::{is_vanilla_extract_file, unwrap_syntax_only};
use crate::visit::DevupVisitor;
use crate::{ExtractOption, ModuleResolver};

/// What a module exports, which marks the components other files select
#[derive(Default)]
pub(crate) struct Exports {
    /// Every name the module exports, in order
    names: Vec<String>,
    /// The names each binding is exported as
    bindings: FxHashMap<String, Vec<String>>,
    /// Whether `export default` exports a value no binding holds
    anonymous_default: bool,
}

impl Exports {
    pub(crate) fn scan(program: &Program<'_>) -> Self {
        let mut exports = Self::default();
        for statement in &program.body {
            match statement {
                Statement::ExportDeclaration(export) => {
                    if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                        for declarator in &declaration.declarations {
                            if let Some(identifier) = declarator.id.get_binding_identifier() {
                                exports.add(identifier.name.as_str(), identifier.name.as_str());
                            }
                        }
                    }
                }
                Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => {
                    for specifier in export
                        .specifiers
                        .iter()
                        .filter(|specifier| !specifier.export_kind.is_type())
                    {
                        exports.add(
                            specifier.local.name().as_str(),
                            specifier.exported.name().as_str(),
                        );
                    }
                }
                Statement::ExportDefaultDeclaration(export) => {
                    match export.declaration.as_expression() {
                        Some(Expression::Identifier(identifier)) => {
                            exports.add(identifier.name.as_str(), "default");
                        }
                        Some(_) => {
                            exports.anonymous_default = true;
                            exports.names.push("default".to_string());
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }
        exports.names.sort_unstable();
        exports.names.dedup();
        exports
    }

    fn add(&mut self, binding: &str, exported: &str) {
        self.names.push(exported.to_string());
        self.bindings
            .entry(binding.to_string())
            .or_default()
            .push(exported.to_string());
    }

    /// How many names the module exports
    pub(crate) const fn count(&self) -> usize {
        self.names.len()
    }

    /// Where the exported `name` stands among the names the module exports
    pub(crate) fn index(&self, name: &str) -> usize {
        self.names
            .binary_search_by(|exported| exported.as_str().cmp(name))
            .unwrap_or_default()
    }

    /// The names the binding `binding` is exported as
    pub(crate) fn of(&self, binding: &str) -> &[String] {
        self.bindings.get(binding).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn bindings(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.bindings.iter()
    }

    pub(crate) const fn has_anonymous_default(&self) -> bool {
        self.anonymous_default
    }
}

/// A styled component another module defines: the classes other styles
/// select it by, and what composing it takes when the build can read it
pub struct ImportedComponent<'a> {
    pub markers: Vec<String>,
    pub definition: Option<StyledDefinition<'a>>,
}

impl ImportedComponent<'_> {
    /// This component in the arena of the file that reads it
    pub fn clone_in<'b>(&self, allocator: &'b Allocator) -> ImportedComponent<'b> {
        ImportedComponent {
            markers: self.markers.clone(),
            definition: self
                .definition
                .as_ref()
                .map(|definition| definition.clone_in(allocator)),
        }
    }

    /// This component as another file can use it: composed only when its
    /// definition holds nothing of the module defining it
    fn shared(self, readable: bool) -> Self {
        Self {
            definition: self
                .definition
                .filter(|definition| readable && definition.portable()),
            ..self
        }
    }
}

type Components<'a> = FxHashMap<String, ImportedComponent<'a>>;

/// The components a file imports, and the modules read to find them
#[derive(Default)]
pub(crate) struct Reading<'a> {
    pub components: Components<'a>,
    pub dependencies: BTreeSet<String>,
}

/// Read the styled components `program` extends, selects or gives a `css`
/// prop, from the modules it imports them from
pub(crate) fn read<'a>(
    ast_builder: &AstBuilder<'a>,
    program: &Program<'a>,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    css_prop: CssProp,
) -> Reading<'a> {
    let Some(resolver) = resolver else {
        return Reading::default();
    };
    let allocator = ast_builder.allocator();
    let mut reader = Reader {
        option,
        resolver,
        allocator,
        modules: FxHashMap::default(),
        reading: vec![filename.to_string()],
        dependencies: BTreeSet::new(),
    };
    let components = reader.imports(program, filename, css_prop, allocator);
    Reading {
        components,
        dependencies: reader.dependencies,
    }
}

/// The components a module exports, by export name
type Module<'a> = Rc<Components<'a>>;

struct Reader<'r, 'a> {
    option: &'r ExtractOption,
    resolver: &'r ModuleResolver,
    allocator: &'a Allocator,
    modules: FxHashMap<String, Option<Module<'a>>>,
    /// The modules being read, which one of them importing another in a
    /// cycle cannot be read through
    reading: Vec<String>,
    dependencies: BTreeSet<String>,
}

impl<'a> Reader<'_, 'a> {
    /// The components of the module `specifier` names, as `importer` imports it
    fn module(&mut self, specifier: &str, importer: &str) -> Option<Module<'a>> {
        let module = (self.resolver)(specifier, importer)?;
        self.dependencies.insert(module.path.clone());
        if let Some(read) = self.modules.get(&module.path) {
            return read.clone();
        }
        if self.reading.contains(&module.path) {
            return None;
        }
        self.reading.push(module.path.clone());
        let read = self.read(&module.path, &module.code).map(Rc::new);
        self.reading.pop();
        self.modules.insert(module.path, read.clone());
        read
    }

    /// The styled components the module at `path` exports, found by compiling
    /// it as its own extraction does
    fn read(&mut self, path: &str, code: &str) -> Option<Components<'a>> {
        self.read_source(path, code, false)
    }

    fn read_source(&mut self, path: &str, code: &str, evaluated: bool) -> Option<Components<'a>> {
        if is_vanilla_extract_file(path) {
            return None;
        }
        let Aliased {
            code: transformed,
            css_prop,
            px,
            ..
        } = transform_import_aliases_with_edits(
            code,
            path,
            &self.option.package,
            &self.option.import_aliases,
        );
        let sub = Allocator::default();
        let ParserReturn {
            mut program,
            fatal_error,
            ..
        } = Parser::new(&sub, &transformed, SourceType::from_path(path).ok()?).parse();
        if fatal_error {
            return None;
        }
        let ast_builder = AstBuilder::new(&sub);
        let inlined = crate::imported_constants::inline_constants(
            &ast_builder,
            &mut program,
            path,
            self.option,
            Some(self.resolver),
            css_prop,
            &px,
        );
        self.dependencies.extend(inlined.dependencies);
        let imports = self.imports(&program, path, css_prop, &sub);
        let mut exports = self.reexports(&program, path);
        let (bucket, global, _) = crate::resolve_css_target(path, self.option);
        let mut visitor = DevupVisitor::new(
            &sub,
            path,
            &self.option.package,
            Vec::new(),
            if global { None } else { Some(bucket) },
        );
        visitor.import_stylex(inlined.stylex_vars, inlined.stylex_themes);
        visitor.import_css(inlined.css_styles);
        visitor.errors.extend(inlined.errors);
        visitor.unknown_bindings(&inlined.unknown);
        visitor.changed_bindings(inlined.changed);
        visitor.takes_css_prop(css_prop);
        visitor.import_styled(imports);
        visitor.visit_program(&mut program);
        let readable = visitor.errors.is_empty()
            && visitor.unknown_parts.is_empty()
            && !visitor.composes_unknown;
        let components = visitor.components(self.allocator);
        let needs_values = components.values().any(|component| {
            component
                .definition
                .as_ref()
                .is_some_and(|definition| !definition.portable())
        });
        if (!readable || needs_values)
            && !evaluated
            && let Some((computed, dependencies)) =
                self.evaluate_values(path, code, &inlined.unknown)
        {
            self.dependencies.extend(dependencies);
            return self.read_source(path, &computed, true);
        }
        exports.extend(
            components
                .into_iter()
                .map(|(name, component)| (name, component.shared(readable))),
        );
        Some(exports)
    }

    fn evaluate_values(
        &self,
        path: &str,
        code: &str,
        unknown: &crate::imported_constants::Unknown,
    ) -> Option<(String, BTreeSet<String>)> {
        let Aliased {
            code: transformed,
            edits: alias_mapping,
            ..
        } = transform_import_aliases_with_edits(
            code,
            path,
            &self.option.package,
            &self.option.import_aliases,
        );
        let allocator = Allocator::default();
        let parsed =
            Parser::new(&allocator, &transformed, SourceType::from_path(path).ok()?).parse();
        if parsed.fatal_error {
            return None;
        }
        let program = parsed.program;
        let scoping = oxc_semantic::SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping();
        let mut sites = ValueSites {
            styled: imported_bindings(&program, &self.option.package)
                .styled
                .into_iter()
                .filter_map(|name| scoping.get_root_binding(name.as_str().into()))
                .collect(),
            scoping: &scoping,
            spans: Vec::new(),
        };
        sites.visit_program(&program);
        let mut index = 0;
        let helper = loop {
            let name = format!("__devupImportedValues{index}");
            if !code.contains(&name) {
                break name;
            }
            index += 1;
        };
        let mut insertions = vec![(
            0,
            format!(
                "import {{ css as {helper} }} from '{}';\n",
                self.option.package
            ),
        )];
        for span in sites.spans {
            insertions.push((
                usize::try_from(span.start).ok()?,
                format!("{helper}({{ __value: ("),
            ));
            insertions.push((usize::try_from(span.end).ok()?, ")})".to_string()));
        }
        insertions.sort_unstable_by_key(|(offset, _)| *offset);
        let mut source = String::new();
        let mut mapping = Vec::new();
        let mut copied = 0;
        for (offset, insertion) in insertions {
            source.push_str(transformed.get(copied..offset)?);
            source.push_str(&insertion);
            mapping.push((offset, offset, insertion.len()));
            copied = offset;
        }
        source.push_str(transformed.get(copied..)?);
        let (computed, replacements, dependencies) = crate::build_time_values::evaluate(
            &source,
            path,
            self.option,
            Some(self.resolver),
            unknown,
        )?;
        let mut result = String::new();
        let mut original = 0;
        let mut before = 0;
        let mut after = 0_usize;
        for (start, end, length) in replacements {
            let replaced_at = after.checked_add(start.checked_sub(before)?)?;
            // Retry unaliased source so computed Emotion numbers keep px semantics.
            let original_start = crate::import_alias_visit::source_offset(
                &alias_mapping,
                crate::import_alias_visit::source_offset(&mapping, start),
            );
            let original_end = crate::import_alias_visit::source_offset(
                &alias_mapping,
                crate::import_alias_visit::source_offset(&mapping, end),
            );
            result.push_str(code.get(original..original_start)?);
            result.push_str(computed.get(replaced_at..replaced_at.checked_add(length)?)?);
            original = original_end;
            before = end;
            after = replaced_at.checked_add(length)?;
        }
        result.push_str(code.get(original..)?);
        Some((result, dependencies))
    }

    /// What the module re-exports from the modules it exports from
    fn reexports(&mut self, program: &Program<'_>, path: &str) -> Components<'a> {
        let mut exports = FxHashMap::default();
        for statement in &program.body {
            match statement {
                Statement::ExportFromDeclaration(export) if !export.export_kind.is_type() => {
                    if let Some(module) = self.module(&export.source.value, path) {
                        for specifier in export
                            .specifiers
                            .iter()
                            .filter(|specifier| !specifier.export_kind.is_type())
                        {
                            if let Some(component) = module.get(specifier.local.name().as_str()) {
                                exports.insert(
                                    specifier.exported.name().to_string(),
                                    component.clone_in(self.allocator),
                                );
                            }
                        }
                    }
                }
                Statement::ExportAllDeclaration(export)
                    if export.exported.is_none() && !export.export_kind.is_type() =>
                {
                    if let Some(module) = self.module(&export.source.value, path) {
                        for (name, component) in
                            module.iter().filter(|(name, _)| *name != "default")
                        {
                            exports
                                .entry(name.clone())
                                .or_insert_with(|| component.clone_in(self.allocator));
                        }
                    }
                }
                _ => {}
            }
        }
        exports
    }

    /// The components `program` imports that it extends, selects or gives a
    /// `css` prop, in the arena of the file that reads them
    fn imports<'b>(
        &mut self,
        program: &Program<'_>,
        importer: &str,
        css_prop: CssProp,
        allocator: &'b Allocator,
    ) -> Components<'b> {
        let imported = imported_bindings(program, &self.option.package);
        let scoping = oxc_semantic::SemanticBuilder::new()
            .build(program)
            .semantic
            .into_scoping();
        let compat = format!("{}/compat", self.option.package);
        let mut used = Used {
            scoping: &scoping,
            css_takers: crate::css_prop::CssTakers::new(program, css_prop, &compat),
            styled: imported.styled,
            css_prop: css_prop != CssProp::Off,
            names: FxHashSet::default(),
        };
        used.visit_program(program);
        crate::style_values::selectors(program, &mut |expression| {
            used.select(expression);
        });
        used.names.extend(
            Exports::scan(program)
                .bindings()
                .map(|(binding, _)| binding.clone()),
        );
        let mut names: Vec<String> = used.names.into_iter().collect();
        names.sort_unstable();
        let mut components = FxHashMap::default();
        for name in names {
            let found = match name.split_once('.') {
                Some((namespace, member)) => imported
                    .bindings
                    .get(namespace)
                    .filter(|(_, export)| export == NAMESPACE)
                    .map(|(source, _)| (source, member)),
                None => imported
                    .bindings
                    .get(&name)
                    .map(|(source, export)| (source, export.as_str())),
            };
            if let Some((source, export)) = found
                && let Some(module) = self.module(source, importer)
                && let Some(component) = module.get(export)
            {
                components.insert(name, component.clone_in(allocator));
            }
        }
        components
    }
}

/// What a namespace import is imported as
const NAMESPACE: &str = "*";

/// What a program imports
struct Imported {
    /// Each binding another module's export, with that module and the export
    bindings: FxHashMap<String, (String, String)>,
    /// The bindings of the package's `styled`
    styled: FxHashSet<String>,
}

fn imported_bindings(program: &Program<'_>, package: &str) -> Imported {
    let mut imported = Imported {
        bindings: FxHashMap::default(),
        styled: FxHashSet::default(),
    };
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        if import.import_kind.is_type() {
            continue;
        }
        let source = import.source.value.as_str();
        let from_package = source.starts_with(package);
        for specifier in import.specifiers.iter().flatten() {
            let (local, export) = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier)
                    if !specifier.import_kind.is_type() =>
                {
                    (
                        specifier.local.name.to_string(),
                        specifier.imported.name().to_string(),
                    )
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                    (specifier.local.name.to_string(), "default".to_string())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                    (specifier.local.name.to_string(), NAMESPACE.to_string())
                }
                ImportDeclarationSpecifier::ImportSpecifier(_) => continue,
            };
            if !from_package {
                imported
                    .bindings
                    .insert(local, (source.to_string(), export));
            } else if export == "styled" {
                imported.styled.insert(local);
            }
        }
    }
    imported
}

/// A binding or a member of one, as the program writes it
fn written_name(expression: &Expression<'_>) -> Option<String> {
    match unwrap_syntax_only(expression) {
        Expression::Identifier(identifier) => Some(identifier.name.to_string()),
        expression => {
            let (object, property) = crate::style_values::component_member(expression)?;
            let Expression::Identifier(object) = unwrap_syntax_only(object) else {
                return None;
            };
            Some(format!("{}.{property}", object.name))
        }
    }
}

/// The bindings a program composes, takes the `css` prop of, or selects
struct Used<'s, 'p> {
    scoping: &'s oxc_semantic::Scoping,
    css_takers: crate::css_prop::CssTakers<'p>,
    styled: FxHashSet<String>,
    css_prop: bool,
    names: FxHashSet<String>,
}

struct ValueSites<'s> {
    scoping: &'s oxc_semantic::Scoping,
    styled: FxHashSet<oxc_syntax::symbol::SymbolId>,
    spans: Vec<Span>,
}

impl ValueSites<'_> {
    fn is_styled(&self, expression: &Expression<'_>) -> bool {
        match unwrap_syntax_only(expression) {
            Expression::Identifier(reference) => reference
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .is_some_and(|symbol| self.styled.contains(&symbol)),
            Expression::StaticMemberExpression(member) => self.is_styled(&member.object),
            Expression::CallExpression(call) => self.is_styled(&call.callee),
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for ValueSites<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.is_styled(&call.callee) {
            self.spans.extend(
                call.arguments
                    .iter()
                    .filter_map(|argument| argument.as_expression())
                    .map(GetSpan::span),
            );
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_tagged_template_expression(
        &mut self,
        tag: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        if self.is_styled(&tag.tag) {
            self.spans
                .extend(tag.quasi.expressions.iter().map(GetSpan::span));
        }
        walk::walk_tagged_template_expression(self, tag);
    }
}

impl Used<'_, '_> {
    fn record(&mut self, reference: &oxc_ast::ast::IdentifierReference<'_>, name: String) {
        if let Some(symbol) = reference
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            && self.scoping.get_root_binding(reference.name) == Some(symbol)
            && self.scoping.symbol_flags(symbol).is_import()
        {
            self.names.insert(name);
        }
    }

    fn select(&mut self, expression: &Expression<'_>) {
        match unwrap_syntax_only(expression) {
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.select(expression);
                }
            }
            Expression::Identifier(reference) => {
                if let Some(name) = written_name(expression) {
                    self.record(reference, name);
                }
            }
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
                if let Some((object, _)) = crate::style_values::component_member(expression)
                    && let Expression::Identifier(reference) = unwrap_syntax_only(object)
                    && let Some(name) = written_name(expression)
                {
                    self.record(reference, name);
                }
            }
            _ => {}
        }
    }
}

impl<'a> Visit<'a> for Used<'_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.css_takers.property(call, |_| false).is_some()
            && let Some(element) = call
                .arguments
                .first()
                .and_then(|argument| argument.as_expression())
        {
            self.select(element);
        }
        let name = match unwrap_syntax_only(&call.callee) {
            Expression::Identifier(callee) if self.styled.contains(callee.name.as_str()) => call
                .arguments
                .first()
                .and_then(|first| first.as_expression()),
            Expression::StaticMemberExpression(member)
                if member.property.name == "withComponent" =>
            {
                Some(&member.object)
            }
            _ => None,
        };
        if let Some(expression) = name {
            self.select(expression);
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        if self.css_prop
            && element.attributes.iter().any(|attribute| {
                matches!(attribute, JSXAttributeItem::Attribute(attribute)
                    if matches!(&attribute.name, JSXAttributeName::Identifier(attribute) if attribute.name == "css"))
            })
        {
            match &element.name {
                JSXElementName::IdentifierReference(name) => {
                    self.record(name, name.name.to_string());
                }
                JSXElementName::MemberExpression(member) => {
                    if let oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(root) =
                        &member.object
                    {
                        self.record(root, format!("{}.{}", root.name, member.property.name));
                    }
                }
                _ => {}
            }
        }
        walk::walk_jsx_opening_element(self, element);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serial_test::serial;

    use super::*;
    use crate::{ExtractOutput, ImportAlias, ResolvedModule, extract_with_modules};

    fn option() -> ExtractOption {
        ExtractOption {
            single_css: true,
            import_aliases: HashMap::from([
                ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
                (
                    "@emotion/styled".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
            ]),
            ..ExtractOption::default()
        }
    }

    fn extract_in(files: &[(&str, &str)], entry: &str) -> Result<ExtractOutput, String> {
        let modules: Vec<(String, String)> = files
            .iter()
            .map(|(path, code)| ((*path).to_string(), (*code).to_string()))
            .collect();
        let resolver = move |specifier: &str, importer: &str| {
            let directory = importer
                .rsplit_once('/')
                .map_or("", |(directory, _)| directory);
            let path = format!("{directory}/{}", specifier.trim_start_matches("./"));
            modules
                .iter()
                .find(|(file, _)| {
                    ["", ".ts", ".tsx", ".css.ts", "/index.ts"]
                        .iter()
                        .any(|extension| format!("{path}{extension}") == *file)
                })
                .map(|(file, code)| ResolvedModule {
                    path: file.clone(),
                    code: code.clone(),
                })
        };
        let code = files.iter().find(|(path, _)| *path == entry).unwrap().1;
        extract_with_modules(entry, code, option(), false, &resolver).map_err(|e| e.to_string())
    }

    fn reset() {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
    }

    fn render(files: &[(&str, &str)], entry: &str) -> String {
        reset();
        match extract_in(files, entry) {
            Ok(output) => {
                let styles: BTreeSet<String> = output
                    .styles
                    .iter()
                    .map(|style| format!("{style:?}"))
                    .collect();
                format!("{}\n{styles:#?}\n{:?}", output.code, output.dependencies)
            }
            Err(error) => error,
        }
    }

    const STYLED: &str = "import styled from '@emotion/styled';\n";

    #[test]
    #[serial]
    fn compiled_namespace_css_when_literal_inlines_composed_styles_and_marker() {
        // Given
        let base = format!("{STYLED}export const Child = styled.span`padding: 4px; color: red;`;");
        let app = format!(
            "{STYLED}import * as UI from './base'; import {{ jsx }} from '@emotion/react/jsx-runtime'; const Anchor = styled.div`${{UI.Child}} {{ margin: 0; }}`; export const P = jsx(UI['Child'], {{ css: {{ color: 'blue' }} }});"
        );
        reset();
        // When
        let output = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
            "/src/app.tsx",
        )
        .unwrap();
        // Then
        assert!(output.code.contains("jsx(\"span\""), "{}", output.code);
        assert!(!output.code.contains("jsx(UI["), "{}", output.code);
        assert!(output.code.contains("className:"), "{}", output.code);
        let styles: BTreeSet<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                crate::ExtractStyleValue::Static(style) => {
                    Some((style.property.as_str(), style.value.as_str()))
                }
                _ => None,
            })
            .collect();
        assert!(
            styles.contains(&("padding", "4px")) && styles.contains(&("color", "blue")),
            "{styles:?}"
        );
        let selector = output
            .styles
            .iter()
            .find_map(|style| {
                let text = format!("{style:?}");
                let (_, rest) = text.split_once("& .")?;
                rest.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
                    .find(|word| word.contains("--"))
                    .map(str::to_string)
            })
            .unwrap();
        assert!(output.code.contains(&selector), "{}", output.code);
    }

    #[rstest]
    #[case("jsx(UI.Child, { css: { color: 'blue' }, as: target })", true)]
    #[case("jsx(UI.Child, { css: { color: 'blue' }, ...props })", true)]
    #[case("(UI) => jsx(UI.Child, { css: { color: 'blue' } })", false)]
    #[serial]
    fn compiled_namespace_css_when_opaque_or_shadowed_has_literal_parity(
        #[case] body: &str,
        #[case] overlap: bool,
    ) {
        // Given
        let base = format!("{STYLED}export const Child = styled.span`color: red;`;");
        let app = format!(
            "import * as UI from './base'; import {{ jsx }} from '@emotion/react/jsx-runtime'; export const P = {body};"
        );
        reset();
        let expected = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
            "/src/app.tsx",
        );
        reset();
        let computed = app.replace("UI.Child", "UI['Child']");
        // When
        let output = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &computed)],
            "/src/app.tsx",
        );
        // Then
        if overlap {
            let expected = expected.unwrap_err();
            let error = output.unwrap_err();
            assert!(expected.starts_with("/src/app.tsx:1:114:"), "{expected}");
            assert!(error.starts_with("/src/app.tsx:1:117:"), "{error}");
            assert_eq!(
                error
                    .split_once(": ")
                    .unwrap()
                    .1
                    .replace("UI[\"Child\"]", "UI.Child"),
                expected.split_once(": ").unwrap().1
            );
        } else {
            let output = output.unwrap();
            assert_eq!(output.styles, expected.unwrap().styles);
            assert!(output.code.contains("jsx(UI["), "{}", output.code);
        }
    }

    #[test]
    #[serial]
    fn imported_computation_when_exact_retains_values_and_dependencies() {
        // Given
        let base = format!(
            "{STYLED}import {{ size }} from './values'; const scaled = n => n * 4; export const Rules = styled.div({{ padding: scaled(size), color: 'red' }});"
        );
        let app = format!(
            "{STYLED}import {{ Rules }} from './base'; export const A = styled(Rules)`color: blue;`;"
        );
        reset();
        // When
        let output = extract_in(
            &[
                ("/src/values.ts", "export const size = 2;"),
                ("/src/base.ts", &base),
                ("/src/app.tsx", &app),
            ],
            "/src/app.tsx",
        )
        .unwrap();
        // Then
        let mut styles: Vec<(&str, &str)> = output
            .styles
            .iter()
            .map(|style| match style {
                crate::ExtractStyleValue::Static(style) => {
                    (style.property.as_str(), style.value.as_str())
                }
                other => panic!("expected static style, got {other:?}"),
            })
            .collect();
        styles.sort_unstable();
        assert_eq!(styles, [("color", "blue"), ("padding", "8px")]);
        assert!(output.code.contains("DevupAs = \"div\""), "{}", output.code);
        assert_eq!(output.dependencies, ["/src/base.ts", "/src/values.ts"]);
    }

    #[test]
    #[serial]
    fn imported_attrs_when_sibling_computation_is_exact_remain_portable() {
        // Given
        let base = format!(
            "{STYLED}const scaled = n => n * 4; const size = 2; export const Rules = styled.div({{ padding: scaled(size) }}); export const Attrs = styled.div.attrs({{ id: 'known' }})`margin: 1px;`;"
        );
        let app = format!(
            "{STYLED}import {{ Attrs }} from './base'; export const A = styled(Attrs)`color: blue;`;"
        );
        reset();
        // When
        let output = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
            "/src/app.tsx",
        )
        .unwrap();
        // Then
        assert!(output.code.contains("DevupAs = \"div\""), "{}", output.code);
        assert!(output.code.contains("id: \"known\""), "{}", output.code);
        assert_eq!(output.styles.len(), 2);
        assert_eq!(output.dependencies, ["/src/base.ts"]);
    }

    #[rstest]
    #[case("styled.div({ padding: scaled(size), color: 'red' })")]
    #[case("styled.div`padding: ${scaled(size)}px; color: red;`")]
    #[case("styled.div(make(size))")]
    #[serial]
    fn imported_computation_when_rules_use_different_static_forms_is_exact(#[case] factory: &str) {
        // Given
        let base = format!(
            "{STYLED}const __devupImportedValues0 = 'reserved'; externalEffect(); const scaled = n => n * 4; const make = n => ({{ padding: n * 4, color: 'red' }}); const size = 2; export const Rules = {factory};"
        );
        let app = format!(
            "{STYLED}import {{ Rules }} from './base'; export const A = styled(Rules)`color: blue;`;"
        );
        reset();
        // When
        let output = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
            "/src/app.tsx",
        )
        .unwrap();
        // Then
        let mut styles: Vec<(&str, &str)> = output
            .styles
            .iter()
            .map(|style| match style {
                crate::ExtractStyleValue::Static(style) => {
                    (style.property.as_str(), style.value.as_str())
                }
                other => panic!("expected static style, got {other:?}"),
            })
            .collect();
        styles.sort_unstable();
        assert_eq!(styles, [("color", "blue"), ("padding", "8px")]);
        assert!(output.code.contains("DevupAs = \"div\""), "{}", output.code);
        assert!(
            !output.code.contains("__devupImportedValues"),
            "{}",
            output.code
        );
    }

    #[rstest]
    #[case("const scaled = n => n * 4; const size = runtimeSize;", "scaled(size)")]
    #[case("const scaled = n => n * 4; let size = 2; size = 3;", "scaled(size)")]
    #[case(
        "const scaled = n => n * 4; const size = Math.random();",
        "scaled(size)"
    )]
    #[case(
        "const scaled = n => { externalEffect(); return n * 4; }; const size = 2;",
        "scaled(size)"
    )]
    #[case(
        "const box = { size: 2 }; box.size = 3; const scaled = n => n * 4;",
        "scaled(box.size)"
    )]
    #[serial]
    fn imported_computation_when_runtime_or_mutating_remains_opaque(
        #[case] scope: &str,
        #[case] value: &str,
    ) {
        // Given
        let base =
            format!("{STYLED}{scope} export const Rules = styled.div({{ padding: {value} }});");
        let app = format!(
            "{STYLED}import {{ Rules }} from './base'; export const A = styled(Rules)`color: blue;`;"
        );
        reset();
        // When
        let output = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
            "/src/app.tsx",
        )
        .unwrap();
        // Then
        assert!(output.code.contains("DevupAs = Rules"), "{}", output.code);
        assert_eq!(output.styles.len(), 1);
        assert_eq!(output.dependencies, ["/src/base.ts"]);
    }

    #[rstest]
    #[case("export const P = styled.div`${UI.Child} { margin: 0; }`;")]
    #[case("export const P = styled.div({ [UI.Child]: { margin: 0 } });")]
    #[case("export const P = styled.div({ [`& ${UI.Child}`]: { margin: 0 } });")]
    #[case("export const P = styled(UI.Child)`color: blue;`;")]
    #[case("export const P = UI.Child.withComponent('section');")]
    #[case(
        "import { jsx } from '@emotion/react/jsx-runtime'; export const P = jsx(UI.Child, { css: { color: 'blue' } });"
    )]
    #[serial]
    fn namespace_computed_when_literal_matches_dot_access(#[case] body: &str) {
        // Given
        let base = format!("{STYLED}export const Child = styled.div`padding: 4px;`;");
        let dot = format!("{STYLED}import * as UI from './base';\n{body}");
        let computed = dot.replace("UI.Child", "UI['Child']");
        reset();
        let expected = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &dot)],
            "/src/app.tsx",
        )
        .unwrap();
        reset();
        // When
        let output = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &computed)],
            "/src/app.tsx",
        )
        .unwrap();
        // Then
        assert_eq!(output.styles, expected.styles);
        assert_eq!(output.dependencies, expected.dependencies);
        assert_eq!(output.code, expected.code);
    }

    #[rstest]
    #[case("export const P = (UI) => styled.div`${UI['Child']} { margin: 0; }`;")]
    #[case("export const P = styled.div`${UI[key]} { margin: 0; }`;")]
    #[case("export const P = styled.div({ [UI[key]]: { margin: 0 } });")]
    #[serial]
    fn namespace_computed_when_shadowed_or_unknown_reports_error(#[case] body: &str) {
        // Given
        let base = format!("{STYLED}export const Child = styled.div`padding: 4px;`;");
        let app = format!(
            "{STYLED}import * as UI from './base'; const Anchor = styled.div`${{UI.Child}} {{ margin: 0; }}`;\n{body}"
        );
        reset();
        // When
        let error = extract_in(
            &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
            "/src/app.tsx",
        )
        .unwrap_err();
        // Then
        assert!(error.starts_with("/src/app.tsx:3:"), "{error}");
        assert!(error.contains("UI["), "{error}");
    }

    #[test]
    #[serial]
    fn namespace_selectors_when_imported_match_named_selectors() {
        let base = format!("{STYLED}export const Child = styled.div`color: red;`;");
        for body in [
            "export const P = styled.div`${Child} { margin: 0; }`;",
            "export const P = styled.div({ [Child]: { margin: 0 } });",
            "export const P = styled.div({ [`&:hover ${Child}`]: { margin: 0 } });",
        ] {
            let named = format!("{STYLED}import {{ Child }} from './base';\n{body}");
            let namespace = format!(
                "{STYLED}import * as UI from './base';\n{}",
                body.replace("Child", "UI.Child")
            );
            reset();
            let named = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &named)],
                "/src/app.tsx",
            )
            .unwrap();
            reset();
            let namespace = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &namespace)],
                "/src/app.tsx",
            )
            .unwrap();
            assert_eq!(namespace.styles, named.styles, "{body}");
            assert_eq!(namespace.dependencies, named.dependencies);
            assert_eq!(namespace.dependencies, ["/src/base.ts"]);
        }
    }

    #[test]
    #[serial]
    fn namespace_css_props_when_imported_match_named_props() {
        let base = format!("{STYLED}export const Child = styled.div`color: red; padding: 4px;`;");
        let header = "/** @jsxImportSource @emotion/react */\n";
        for body in [
            "export const C = () => <Child css={{ color: 'blue' }} />;",
            "export const C = () => <Child css={[{ color: 'green' }, { color: 'blue' }]} />;",
            "export const C = () => <Child as='a' css={{ color: 'blue' }} />;",
            "import { jsx } from 'react/jsx-runtime'; export const C = () => jsx(Child, { css: { color: 'blue' } });",
        ] {
            let named = format!("{header}import {{ Child }} from './base';\n{body}");
            let namespace = format!(
                "{header}import * as UI from './base';\n{}",
                body.replace("Child", "UI.Child")
            );
            reset();
            let named = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &named)],
                "/src/app.tsx",
            );
            reset();
            let namespace = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &namespace)],
                "/src/app.tsx",
            );
            match (named, namespace) {
                (Ok(named), Ok(namespace)) => {
                    assert_eq!(namespace.styles, named.styles, "{body}");
                    let colors: Vec<&str> = namespace
                        .styles
                        .iter()
                        .filter_map(|style| match style {
                            crate::ExtractStyleValue::Static(style)
                                if style.property == "color" =>
                            {
                                Some(style.value.as_str())
                            }
                            _ => None,
                        })
                        .collect();
                    assert_eq!(colors, ["blue"]);
                    assert_eq!(namespace.dependencies, named.dependencies);
                    assert_eq!(
                        namespace.code.replace("import * as UI", "import { Child }"),
                        named.code
                    );
                }
                (Err(named), Err(namespace)) => {
                    assert!(namespace.starts_with("/src/app.tsx:"));
                    let named_location =
                        named.split_once(": ").unwrap().0.rsplit_once(':').unwrap();
                    let namespace_location = namespace
                        .split_once(": ")
                        .unwrap()
                        .0
                        .rsplit_once(':')
                        .unwrap();
                    assert_eq!(namespace_location.0, named_location.0);
                    assert_eq!(
                        namespace_location.1.parse::<usize>().unwrap(),
                        named_location.1.parse::<usize>().unwrap() + 3
                    );
                    assert_eq!(
                        namespace
                            .split_once(": ")
                            .unwrap()
                            .1
                            .replace("UI.Child", "Child"),
                        named.split_once(": ").unwrap().1
                    );
                }
                (named, namespace) => panic!("{body}: {named:?} != {namespace:?}"),
            }
        }
    }

    #[test]
    #[serial]
    fn namespace_reexports_when_selected_preserve_defining_markers() {
        let base = format!("{STYLED}export const Child = styled.div`color: red;`;");
        let forward = "export { Child as Renamed } from './base';";
        let named = format!(
            "{STYLED}import {{ Renamed }} from './forward'; export const P = styled.div`${{Renamed}} {{ margin: 0; }}`;"
        );
        let namespace = format!(
            "{STYLED}import * as UI from './forward'; export const P = styled.div`${{UI.Renamed}} {{ margin: 0; }}`;"
        );
        reset();
        let named = extract_in(
            &[
                ("/src/base.ts", &base),
                ("/src/forward.ts", forward),
                ("/src/app.tsx", &named),
            ],
            "/src/app.tsx",
        )
        .unwrap();
        reset();
        let namespace = extract_in(
            &[
                ("/src/base.ts", &base),
                ("/src/forward.ts", forward),
                ("/src/app.tsx", &namespace),
            ],
            "/src/app.tsx",
        )
        .unwrap();
        assert_eq!(namespace.styles, named.styles);
        assert_eq!(namespace.dependencies, ["/src/base.ts", "/src/forward.ts"]);
    }

    #[test]
    #[serial]
    fn namespace_css_props_when_shadowed_keep_the_runtime_component() {
        let base = format!("{STYLED}export const Child = styled.div`color: red; padding: 4px;`;");
        for body in [
            "export const C = (UI) => <UI.Child css={{ color: 'blue' }} />;",
            "import { jsx } from 'react/jsx-runtime'; export const C = (UI) => jsx(UI.Child, { css: { color: 'blue' } });",
        ] {
            let app = format!(
                "/** @jsxImportSource @emotion/react */\nimport * as UI from './base';\nexport const A = <UI.Child css={{{{ padding: 8 }}}} />;\n{body}"
            );
            reset();
            let output = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
                "/src/app.tsx",
            )
            .unwrap();
            assert!(output.code.contains("UI.Child"));
            assert_eq!(output.styles.len(), 3);
            assert!(
                output
                    .styles
                    .iter()
                    .all(|style| !format!("{style:?}").contains("4px"))
            );
        }
    }

    #[test]
    #[serial]
    fn namespace_selectors_when_shadowed_are_not_imported_markers() {
        let base = format!("{STYLED}export const Child = styled.div`color: red;`;");
        for body in [
            "export const P = (UI) => styled.div`${UI.Child} { margin: 0; }`;",
            "export const P = (UI) => styled.div({ [UI.Child]: { margin: 0 } });",
            "export const P = (UI) => styled.div({ [`& ${UI.Child}`]: { margin: 0 } });",
        ] {
            let app = format!(
                "{STYLED}import * as UI from './base';\nconst Anchor = styled.div`${{UI.Child}} {{ padding: 0; }}`;\n{body}"
            );
            reset();
            let error = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
                "/src/app.tsx",
            )
            .unwrap_err();
            assert!(error.starts_with("/src/app.tsx:"), "{error}");
            assert!(error.contains("UI.Child"), "{error}");
        }
    }

    #[test]
    #[serial]
    fn namespace_css_props_when_no_value_export_exists_do_not_inherit_styles() {
        let base = format!("{STYLED}export const Child = styled.div`padding: 4px;`;");
        for import in [
            "import * as UI from './base';",
            "import type * as UI from './base';",
            "import { Child as UI } from './base';",
            "import { type Child as UI } from './base';",
        ] {
            let member = if import == "import * as UI from './base';" {
                "Missing"
            } else {
                "Child"
            };
            let app = format!(
                "/** @jsxImportSource @emotion/react */\n{import}\nexport const C = () => <UI.{member} css={{{{ color: 'blue' }}}} />;"
            );
            reset();
            let output = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
                "/src/app.tsx",
            )
            .unwrap();
            assert!(output.code.contains(&format!("UI.{member}")));
            assert_eq!(output.styles.len(), 1);
        }
    }

    #[test]
    #[serial]
    fn module_definitions_when_inlining_reports_invalid_locals_are_opaque() {
        let allocator = Allocator::default();
        let option = option();
        let resolver = |_: &str, _: &str| None;
        let mut reader = Reader {
            option: &option,
            resolver: &resolver,
            allocator: &allocator,
            modules: FxHashMap::default(),
            reading: Vec::new(),
            dependencies: BTreeSet::new(),
        };
        let code = format!(
            "{STYLED}import {{ css }} from '@devup-ui/react'; export const Child = styled.div`padding: 4px;`; export function bad() {{ const local = {{ color: 'red' }}; expose(local); return css(local); }}"
        );
        reset();
        let module = reader.read("/src/base.ts", &code).unwrap();
        assert!(module["Child"].definition.is_none());
        assert_eq!(module["Child"].markers.len(), 1);
    }

    #[test]
    #[serial]
    fn namespace_type_reexports_when_given_css_do_not_become_value_definitions() {
        let base = format!("{STYLED}export const Child = styled.div`padding: 4px;`;");
        let app = "/** @jsxImportSource @emotion/react */\nimport * as UI from './forward'; export const C = () => <UI.Child css={{ color: 'blue' }} />;";
        for forward in [
            "export type { Child } from './base';",
            "export { type Child } from './base';",
            "export type * from './base';",
        ] {
            reset();
            let output = extract_in(
                &[
                    ("/src/base.ts", &base),
                    ("/src/forward.ts", forward),
                    ("/src/app.tsx", app),
                ],
                "/src/app.tsx",
            )
            .unwrap();
            assert!(output.code.contains("<UI.Child"));
            assert_eq!(output.styles.len(), 1);
        }
    }

    #[test]
    #[serial]
    fn namespace_selectors_when_not_value_members_report_the_source_location() {
        let base = format!("{STYLED}export const Child = styled.div`color: red;`;");
        for (import, selected) in [
            ("import * as UI from './base';", "UI.Missing"),
            ("import type * as UI from './base';", "UI.Child"),
            ("import { Child as UI } from './base';", "UI.Child"),
        ] {
            let app = format!(
                "{STYLED}{import}\nexport const P = styled.div`${{{selected}}} {{ margin: 0; }}`;"
            );
            reset();
            let error = extract_in(
                &[("/src/base.ts", &base), ("/src/app.tsx", &app)],
                "/src/app.tsx",
            )
            .unwrap_err();
            assert!(error.starts_with("/src/app.tsx:3:31:"), "{error}");
            assert!(error.contains(selected), "{error}");
        }
    }
    const BASE_BODY: &str = r"export const Base = styled.div`color: red; padding: 4px;`;
export const Dyn = styled.button`font-size: ${(p) => (p.big ? '20px' : '10px')};`;
export const Pick = styled.p.withConfig({ shouldForwardProp: (prop) => prop !== 'tone' })`color: ${(p) => p.tone};`;
export default styled.span`margin: 1px;`;
";

    fn base_module() -> String {
        format!("{STYLED}{BASE_BODY}")
    }

    fn consumers<'a>(base: &'a str, bodies: &[&'a str]) -> Vec<String> {
        bodies
            .iter()
            .map(|body| {
                render(
                    &[
                        ("/src/base.ts", base),
                        (
                            "/src/app.tsx",
                            &format!(
                                "{STYLED}import Span, {{ Base, Dyn, Pick }} from './base';\n{body}"
                            ),
                        ),
                    ],
                    "/src/app.tsx",
                )
            })
            .collect()
    }

    const COMPOSITIONS: &[&str] = &[
        "export const A = styled(Base)`color: blue;`;",
        "export const A = styled(Base)({ color: 'blue', padding: 8 });",
        "export const A = styled(Base).attrs({ role: 'note' })`color: blue;`;",
        "export const A = styled(Base).attrs((p) => ({ title: p.title }))`color: blue;`;",
        "export const A = styled(Dyn)`color: blue;`;",
        "export const A = styled(Pick)`margin: 0;`;",
        "export const A = styled(Span)`color: blue;`;",
        "export const A = styled(Base)`color: blue;`;\nexport const B = styled(A)`padding: 0;`;",
        "export const W = Dyn.withComponent('section');\nexport const V = Base.withComponent(Span);",
        "export const A = styled(Base)`color: blue;`;\nexport const C = () => <A foo=\"1\" />;",
    ];

    #[test]
    #[serial]
    fn imported_components_compose_like_local_ones() {
        assert_debug_snapshot!(consumers(&base_module(), COMPOSITIONS));
    }

    const DEFINITIONS: [(&str, &str); 4] = [
        (
            "Base",
            "export const Base = styled.div`color: red; padding: 4px;`;\n",
        ),
        (
            "Dyn",
            "export const Dyn = styled.button`font-size: ${(p) => (p.big ? '20px' : '10px')};`;\n",
        ),
        (
            "Pick",
            "export const Pick = styled.p.withConfig({ shouldForwardProp: (prop) => prop !== 'tone' })`color: ${(p) => p.tone};`;\n",
        ),
        ("Span", "export default styled.span`margin: 1px;`;\n"),
    ];

    #[test]
    #[serial]
    fn imported_components_give_no_style_local_ones_do_not() {
        let pragma = "/** @jsxImportSource @emotion/react */\n";
        let css_props = [
            "export const C = () => <Base css={{ color: 'blue' }} foo=\"1\" />;",
            "export const C = () => <Base css={[{ color: 'blue' }, { margin: 2 }]} />;",
        ];
        for body in COMPOSITIONS.iter().chain(&css_props) {
            let header = if body.contains("css=") { pragma } else { "" };
            let used: Vec<&(&str, &str)> = DEFINITIONS
                .iter()
                .filter(|(name, _)| body.contains(name))
                .collect();
            let module: String = used.iter().map(|(_, definition)| *definition).collect();
            let named: Vec<&str> = used
                .iter()
                .map(|(name, _)| *name)
                .filter(|name| *name != "Span")
                .collect();
            let default = if used.iter().any(|(name, _)| *name == "Span") {
                "Span, "
            } else {
                ""
            };
            reset();
            let imported = extract_in(
                &[
                    ("/src/base.ts", &format!("{STYLED}{module}")),
                    (
                        "/src/app.tsx",
                        &format!(
                            "{header}{STYLED}import {default}{{ {} }} from './base';\n{body}",
                            named.join(", ")
                        ),
                    ),
                ],
                "/src/app.tsx",
            )
            .unwrap();
            reset();
            let local_source = format!(
                "{header}{STYLED}{}\n{body}",
                module.replace("export default styled.span", "const Span = styled.span")
            );
            let local = extract_in(&[("/src/app.tsx", &local_source)], "/src/app.tsx").unwrap();
            assert!(imported.styles.is_subset(&local.styles), "{body}");
            assert_eq!(imported.dependencies, ["/src/base.ts"]);
        }
    }
    #[test]
    #[serial]
    fn imported_components_give_css_props_what_local_ones_give() {
        let pragma = "/** @jsxImportSource @emotion/react */\n";
        let bodies = [
            "export const C = () => <Base css={{ color: 'blue' }} foo=\"1\" />;",
            "export const C = () => <Base css={[{ color: 'blue' }, { margin: 2 }]} as=\"a\" />;",
            "export const C = () => <Dyn css={{ color: 'blue' }} />;",
            "export const C = ({ rest }) => <Base css={{ margin: 1 }} {...rest} />;",
        ];
        let outputs: Vec<String> = bodies
            .iter()
            .map(|body| {
                render(
                    &[
                        ("/src/base.ts", &base_module()),
                        (
                            "/src/app.tsx",
                            &format!("{pragma}import {{ Base, Dyn }} from './base';\n{body}"),
                        ),
                    ],
                    "/src/app.tsx",
                )
            })
            .collect();
        assert_debug_snapshot!(outputs);
    }

    #[test]
    #[serial]
    fn imported_components_are_selected_by_the_class_their_module_gives() {
        let bodies = [
            "export const P = styled.div`${Base} { margin: 0; } ${Dyn}:hover & { color: red; }`;",
            "export const P = styled.div({ [Base]: { margin: 0 }, [`&:focus ${Dyn}`]: { color: 'red' } });",
            "export const P = styled.div`${Span} + ${Pick} { margin: 0; }`;",
            "export const P = styled.div`${Base} { margin: 0; }`;\nexport const Q = styled(Base)`color: blue;`;",
        ];
        assert_debug_snapshot!(consumers(&base_module(), &bodies));

        let mid = format!(
            "{STYLED}import {{ Base }} from './base';\nexport const Mid = styled(Base)`width: 1px;`;\nconst Local = styled(Mid)`height: 1px;`;\nexport {{ Local as Renamed }};\nexport default Mid;"
        );
        let app = format!(
            "{STYLED}import {{ Base }} from './base';\nimport Top, {{ Mid, Renamed }} from './mid';\nexport const P = styled.div`${{Base}} {{ margin: 0; }} ${{Mid}} {{ color: red; }} ${{Renamed}} {{ top: 0; }} ${{Top}} {{ left: 0; }}`;"
        );
        assert_debug_snapshot!(render(
            &[
                ("/src/base.ts", &base_module()),
                ("/src/mid.ts", &mid),
                ("/src/app.tsx", &app),
            ],
            "/src/app.tsx",
        ));
    }

    #[test]
    #[serial]
    fn every_file_computes_the_class_of_a_component_from_its_module_and_name() {
        let files = [
            ("/src/base.ts", base_module()),
            (
                "/src/app.tsx",
                format!(
                    "{STYLED}import {{ Dyn }} from './base';\nexport const P = styled.div`${{Dyn}} {{ margin: 0; }}`;"
                ),
            ),
        ];
        let files: Vec<(&str, &str)> = files.iter().map(|(p, c)| (*p, c.as_str())).collect();
        reset();
        let _ = css::file_map::get_file_num_by_filename("/src/app.tsx");
        let app = extract_in(&files, "/src/app.tsx").unwrap();
        let base = extract_in(&files, "/src/base.ts").unwrap();
        let marker = |text: &str| {
            text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
                .find(|word| word.contains("--"))
                .unwrap()
                .to_string()
        };
        let selector = app
            .styles
            .iter()
            .find_map(|style| {
                format!("{style:?}")
                    .split_once("& .")
                    .map(|(_, rest)| marker(rest))
            })
            .unwrap();
        assert!(base.code.contains(&selector), "{selector}: {}", base.code);
    }

    #[test]
    #[serial]
    fn re_exports_and_namespaces_reach_the_defining_module() {
        let reexports: &[(&str, &str)] = &[
            ("/src/base.ts", &base_module_static()),
            (
                "/src/star.ts",
                "export * from './base';\nexport * as everything from './base';\nexport const Base = 1;",
            ),
            (
                "/src/named.ts",
                "export { Base as Renamed, Dyn, Missing } from './base';\nexport { default as Span } from './base';",
            ),
            (
                "/src/forward.ts",
                "import { Dyn } from './base';\nexport { Dyn };\nexport type { Dyn as DynType };",
            ),
            (
                "/src/chain.ts",
                "export * from './star';\nexport * from './named';",
            ),
        ];
        let bodies = [
            "import { Dyn } from './star';\nexport const A = styled(Dyn)`color: blue;`;",
            "import { Base } from './star';\nexport const A = styled(Base)`color: blue;`;",
            "import { Renamed, Span } from './named';\nexport const A = styled(Renamed)`color: blue;`;\nexport const B = styled(Span)`color: blue;`;",
            "import { Dyn } from './forward';\nexport const A = styled(Dyn)`color: blue;`;",
            "import { Renamed, Dyn } from './chain';\nexport const A = styled(Renamed)`color: blue;`;\nexport const B = styled(Dyn)`color: blue;`;",
            "import * as UI from './named';\nexport const A = styled(UI.Renamed)`color: blue;`;\nexport const W = UI.Dyn.withComponent('a');\nexport const N = styled(UI.Nope)`color: red;`;",
            "import { Base } from './named';\nexport const A = styled(Base)`color: blue;`;",
            "import { Dyn, Span } from './star';\nexport const A = styled(Dyn)`color: blue;`;\nexport const B = styled(Span)`color: blue;`;",
        ];
        let outputs: Vec<String> = bodies
            .iter()
            .map(|body| {
                let app = format!("{STYLED}{body}");
                let mut files = reexports.to_vec();
                files.push(("/src/app.tsx", &app));
                render(&files, "/src/app.tsx")
            })
            .collect();
        assert_debug_snapshot!(outputs);
    }

    fn base_module_static() -> String {
        base_module()
    }

    #[test]
    #[serial]
    fn imported_components_the_build_cannot_read_stay_runtime_components() {
        let modules: &[(&str, &str)] = &[
            (
                "/src/link.ts",
                "import styled from '@emotion/styled';\nimport Link from 'next/link';\nexport const L = styled(Link)`color: red;`;\nexport const M = styled(L)`margin: 0;`;",
            ),
            (
                "/src/attrs.ts",
                "import styled from '@emotion/styled';\nconst handler = () => 1;\nexport const H = styled.div.attrs({ onClick: handler })`color: red;`;",
            ),
            (
                "/src/function.ts",
                "import styled from '@emotion/styled';\nfunction scaled(n) { return n * 2; }\nexport const D = styled.div`margin: ${(p) => scaled(p.x)}px;`;",
            ),
            (
                "/src/global.ts",
                "import styled from '@emotion/styled';\nexport const E = styled.div({ display: process.env.MODE });",
            ),
            (
                "/src/closed.ts",
                "import styled from '@emotion/styled';\nexport const G = styled.div`margin: ${(p) => Number(p.x)}px;`;\nexport const T = styled.div({ color: ['red', 'blue'] });\nexport const U = styled.div`color: ${(p: Props) => p.tone};`;\ntype Props = { tone: string };",
            ),
            (
                "/src/plain.ts",
                "export const Base = 5;\nexport function fn() {}\nexport const { a, b } = { a: 1, b: 2 };\nexport default function () {}\nexport type { Props };\nexport { type Props as P };",
            ),
            (
                "/src/bad.ts",
                "import styled from '@emotion/styled';\nexport const Good = styled.div`color: red;`;\nexport const Bad = styled.div;",
            ),
            (
                "/src/cycle-a.ts",
                "import styled from '@emotion/styled';\nimport { B } from './cycle-b';\nexport const A = styled(B)`color: red;`;",
            ),
            (
                "/src/cycle-b.ts",
                "import styled from '@emotion/styled';\nimport { A } from './cycle-a';\nexport const B = styled(A)`color: blue;`;",
            ),
            (
                "/src/theme.css.ts",
                "import { style } from '@vanilla-extract/css';\nexport const base = style({ color: 'red' });",
            ),
            ("/src/data.json", "{}"),
            ("/src/broken.ts", "export const = ;"),
            (
                "/src/types.ts",
                "import styled from '@emotion/styled';\nexport const Base = styled.div`color: red;`;",
            ),
        ];
        let bodies = [
            "import { L, M } from './link';\nexport const A = styled(L)`color: blue;`;\nexport const B = styled(M)`color: blue;`;",
            "import { H } from './attrs';\nimport { D } from './function';\nimport { E } from './global';\nimport { G, T, U } from './closed';\nexport const A = styled(H)`color: blue;`;\nexport const B = styled(D)`color: blue;`;\nexport const C = styled(G)`color: blue;`;\nexport const F = styled(E)`color: blue;`;\nexport const I = styled(T)`color: blue;`;\nexport const J = styled(U)`color: blue;`;",
            "import Anonymous, { Base, fn, a, P } from './plain';\nexport const A = styled(Base)`color: blue;`;\nexport const B = styled(Anonymous)`color: blue;`;\nexport const C = styled(fn)`color: blue;`;\nexport const D = styled(a)`color: blue;`;\nexport const E = styled(P)`color: blue;`;",
            "import { Good, Bad } from './bad';\nexport const A = styled(Good)`color: blue;`;\nexport const B = styled.div`${Good} { color: red; }`;",
            "import { A } from './cycle-a';\nexport const App = styled(A)`color: blue;`;",
            "import { base } from './theme.css';\nimport { Data } from './data.json';\nimport { Broken } from './broken';\nimport { Missing } from './missing';\nimport { Nope } from './types';\nexport const A = styled(Data)`color: blue;`;\nexport const B = styled(Broken)`color: blue;`;\nexport const C = styled(Missing)`color: blue;`;\nexport const D = styled(Nope)`color: blue;`;\nexport const E = styled(base)`color: blue;`;",
            "import type { Base } from './types';\nimport { type Base as Other } from './types';\nexport const A = styled(Base)`color: blue;`;\nexport const B = styled(Other)`color: blue;`;",
            "import { Base } from './types';\nexport const A = styled(Base)`color: blue;`;",
            "import * as types from './types';\nexport const A = styled(types.deep.Base)`color: blue;`;\nexport const B = styled(types['Base'])`color: blue;`;",
        ];
        let outputs: Vec<String> = bodies
            .iter()
            .map(|body| {
                let app = format!("{STYLED}{body}");
                let mut files = modules.to_vec();
                files.push(("/src/app.tsx", &app));
                render(&files, "/src/app.tsx")
            })
            .collect();
        assert_debug_snapshot!(outputs);
    }

    #[test]
    #[serial]
    fn exports_name_what_a_module_gives_other_files() {
        let code = "import styled from '@emotion/styled';\nexport const A = 1, { b } = {};\nconst C = 2;\nexport { C, C as D };\nexport default C;\nexport function f() {}\nexport type { C as T };";
        let allocator = Allocator::default();
        let program = Parser::new(&allocator, code, SourceType::ts())
            .parse()
            .program;
        let exports = Exports::scan(&program);
        assert_eq!(exports.count(), 4);
        assert_eq!(exports.of("C"), ["C", "D", "default"]);
        assert_eq!(exports.of("A"), ["A"]);
        assert_eq!(exports.of("missing"), [] as [String; 0]);
        assert_eq!(exports.index("D"), 2);
        assert!(!exports.has_anonymous_default());
        let code = "export default styled.div`color: red;`;";
        let program = Parser::new(&allocator, code, SourceType::ts())
            .parse()
            .program;
        let exports = Exports::scan(&program);
        assert!(exports.has_anonymous_default());
        assert_eq!(exports.index("default"), 0);
    }
}
#[cfg(test)]
#[path = "imported_styled_coverage_tests.rs"]
mod coverage_tests;
