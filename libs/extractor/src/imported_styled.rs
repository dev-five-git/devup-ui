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
use oxc_span::SourceType;
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
        if is_vanilla_extract_file(path) {
            return None;
        }
        let Aliased {
            code: transformed,
            css_prop,
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
        visitor.unknown_bindings(&inlined.unknown);
        visitor.changed_bindings(inlined.changed);
        visitor.takes_css_prop(css_prop);
        visitor.import_styled(imports);
        visitor.visit_program(&mut program);
        let readable = visitor.errors.is_empty()
            && visitor.unknown_parts.is_empty()
            && !visitor.composes_unknown;
        exports.extend(
            visitor
                .components(self.allocator)
                .into_iter()
                .map(|(name, component)| (name, component.shared(readable))),
        );
        Some(exports)
    }

    /// What the module re-exports from the modules it exports from
    fn reexports(&mut self, program: &Program<'_>, path: &str) -> Components<'a> {
        let mut exports = FxHashMap::default();
        for statement in &program.body {
            match statement {
                Statement::ExportFromDeclaration(export) => {
                    if let Some(module) = self.module(&export.source.value, path) {
                        for specifier in &export.specifiers {
                            if let Some(component) = module.get(specifier.local.name().as_str()) {
                                exports.insert(
                                    specifier.exported.name().to_string(),
                                    component.clone_in(self.allocator),
                                );
                            }
                        }
                    }
                }
                Statement::ExportAllDeclaration(export) if export.exported.is_none() => {
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
        let mut used = Used {
            styled: imported.styled,
            css_prop: css_prop != CssProp::Off,
            names: FxHashSet::default(),
        };
        used.visit_program(program);
        crate::style_values::selectors(program, &mut |expression| {
            if let Expression::Identifier(identifier) = expression {
                used.names.insert(identifier.name.to_string());
            }
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
        Expression::StaticMemberExpression(member) => match &member.object {
            Expression::Identifier(object) => {
                Some(format!("{}.{}", object.name, member.property.name))
            }
            _ => None,
        },
        _ => None,
    }
}

/// The bindings a program composes, takes the `css` prop of, or selects
struct Used {
    styled: FxHashSet<String>,
    css_prop: bool,
    names: FxHashSet<String>,
}

impl<'a> Visit<'a> for Used {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let name = match unwrap_syntax_only(&call.callee) {
            Expression::Identifier(callee) if self.styled.contains(callee.name.as_str()) => call
                .arguments
                .first()
                .and_then(|first| first.as_expression())
                .and_then(written_name),
            Expression::StaticMemberExpression(member)
                if member.property.name == "withComponent" =>
            {
                written_name(&member.object)
            }
            _ => None,
        };
        self.names.extend(name);
        walk::walk_call_expression(self, call);
    }

    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        if self.css_prop
            && let JSXElementName::IdentifierReference(name) = &element.name
            && element.attributes.iter().any(|attribute| {
                matches!(attribute, JSXAttributeItem::Attribute(attribute)
                    if matches!(&attribute.name, JSXAttributeName::Identifier(attribute) if attribute.name == "css"))
            })
        {
            self.names.insert(name.name.to_string());
        }
        walk::walk_jsx_opening_element(self, element);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use insta::assert_debug_snapshot;
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
                "/src/free.ts",
                "import styled from '@emotion/styled';\nconst handler = () => 1;\nexport const H = styled.div.attrs({ onClick: handler })`color: red;`;\nexport const C = styled.div({ color: window.dark ? 'red' : 'blue' });\nexport const E = styled.div({ display: window.mode });\nexport const T = styled.div({ color: ['red', 'blue'] });\nexport const U = styled.div({ color: (p: Props) => p.tone });\ntype Props = { tone: string };",
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
            "import { H, C, E, T, U } from './free';\nexport const A = styled(H)`color: blue;`;\nexport const B = styled(C)`color: blue;`;\nexport const D = styled(E)`color: blue;`;\nexport const F = styled(T)`color: blue;`;\nexport const G = styled(U)`color: blue;`;",
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
