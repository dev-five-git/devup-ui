//! The Devup UI imports of a file, rewritten to React Native's components.
//!
//! An imported component keeps its local name, now bound to the React Native
//! component it compiles to: `import { Box } from '@devup-ui/react'` becomes
//! `import { View as Box } from 'react-native'`.

use oxc_ast::ast::{
    ImportDeclaration, ImportDeclarationSpecifier, ImportSpecifier, ModuleExportName, Program,
    Statement,
};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::errors::{Problem, compat_theme_problem, reanimated_problem, theme_api};
use super::{COMPAT_PACKAGES, NativeTarget, REACT_NATIVE, RUNTIME, Replacement};
use crate::component::ExportVariableKind;

/// The Devup UI components a file imports, and how its imports change
#[derive(Default)]
pub(super) struct Imports {
    /// Each component's binding: its kind and local name
    pub components: FxHashMap<SymbolId, (ExportVariableKind, String)>,
    /// The bindings of `css`, whose calls compile to style objects
    pub css: FxHashSet<SymbolId>,
    /// The bindings of `styled`, whose calls compile to components
    pub styled: FxHashSet<SymbolId>,
    /// The bindings of `styled` and `css` imported from a compat package, whose
    /// function parts read the theme of its `ThemeProvider`
    pub compat: FxHashSet<SymbolId>,
    /// The bindings of `keyframes`, whose calls compile to the keyframes
    /// react-native-reanimated reads
    pub keyframes: FxHashSet<SymbolId>,
    pub replacements: Vec<Replacement>,
    pub errors: Vec<(u32, String)>,
    /// Whether the file imports the theme API of the runtime
    pub theme_api: bool,
    /// Where the stylesheet is declared: before the first statement that is
    /// not an import
    pub sheet_at: Option<usize>,
}

/// The React Native component a Devup UI component renders
pub(super) const fn native_component(kind: &ExportVariableKind) -> &'static str {
    match kind {
        ExportVariableKind::Box
        | ExportVariableKind::Flex
        | ExportVariableKind::VStack
        | ExportVariableKind::Center
        | ExportVariableKind::Grid => "View",
        ExportVariableKind::Text => "Text",
        ExportVariableKind::Image => "Image",
        ExportVariableKind::Input => "TextInput",
        ExportVariableKind::Button => "Pressable",
    }
}

/// The theme API, which the runtime of React Native implements
fn is_theme_api(name: &str) -> bool {
    matches!(name, "useTheme" | "getTheme" | "setTheme")
}

/// Where an imported name of the package goes on React Native
enum Destination {
    /// A React Native component, under this specifier
    Native(String),
    /// The runtime, under this specifier
    Runtime(String),
    /// The package, as it is: a name only types read
    Kept,
    /// Nowhere: the build compiles each use of the name away
    Dropped,
}

/// Why an export of the package cannot be imported on React Native, or `None`
/// for one that can (a component, the theme API with the runtime, or a name
/// only types read)
fn unsupported(name: &str, target: &NativeTarget) -> Option<Problem> {
    match name {
        "Grid" => Some(Problem::new(
            "React Native has no grid layout",
            "lay the items out with `Flex` and `flexWrap`",
        )),
        name if is_theme_api(name) => (!target.runtime).then(|| theme_api(name)),
        "ThemeScript" | "initTheme" | "globalCss" => Some(Problem::new(
            "React Native has no document and no global stylesheet",
            "remove it from the React Native build",
        )),
        "keyframes" => (!target.reanimated).then(reanimated_problem),
        "stylex" => Some(Problem::new(
            "StyleX has no React Native output",
            "write the styles as props of a Devup UI component",
        )),
        _ => None,
    }
}

/// Read the imports of `package` in `program`
pub(super) fn read(
    program: &Program<'_>,
    code: &str,
    package: &str,
    target: &NativeTarget,
) -> Imports {
    let mut imports = Imports::default();
    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(declaration) => {
                let source = declaration.source.value.as_str().unwrap_or_default();
                let compat = COMPAT_PACKAGES.contains(&source);
                if (source == package || compat) && !declaration.import_kind.is_type() {
                    let package = if compat { source } else { package };
                    imports.declaration(declaration, code, (package, compat), target);
                }
            }
            statement => {
                imports
                    .sheet_at
                    .get_or_insert_with(|| statement.span().start as usize);
            }
        }
    }
    imports
}

impl Imports {
    fn declaration(
        &mut self,
        declaration: &ImportDeclaration<'_>,
        code: &str,
        (package, compat): (&str, bool),
        target: &NativeTarget,
    ) {
        let mut natives = Vec::new();
        let mut runtime = Vec::new();
        let mut kept = Vec::new();
        for specifier in declaration.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) if compat => {
                    // The default export of a compat package is its `styled`
                    self.compat_styled(specifier.local.symbol_id.get());
                }
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    let destination = if compat {
                        self.compat_specifier(specifier, package, target)
                    } else {
                        self.specifier(specifier, package, target)
                    };
                    match destination {
                        Destination::Native(native) => natives.push(native),
                        Destination::Runtime(name) => runtime.push(name),
                        Destination::Kept => kept.push(specifier_code(specifier, code)),
                        Destination::Dropped => {}
                    }
                }
                specifier => self.errors.push((
                    specifier.span().start,
                    Problem::new(
                        "the build reads only components imported by name",
                        format!("import each component by name, such as `import {{ Box }} from '{package}'`"),
                    )
                    .import(specifier.span().source_text(code), package),
                )),
            }
        }
        let dropped = natives.len() + runtime.len() + kept.len()
            < declaration.specifiers.iter().flatten().count();
        if natives.is_empty() && runtime.is_empty() && !dropped {
            return;
        }
        self.theme_api |= !runtime.is_empty() && !compat;
        let source = declaration.source.span.source_text(code).to_string();
        let text: Vec<String> = [
            (natives, format!("\"{REACT_NATIVE}\"")),
            (runtime, format!("\"{RUNTIME}\"")),
            (kept, source),
        ]
        .into_iter()
        .filter(|(names, _)| !names.is_empty())
        .map(|(names, source)| format!("import {{ {} }} from {source};", names.join(", ")))
        .collect();
        let span = declaration.span;
        self.replacements
            .push((span.start as usize, span.end as usize, text.join("\n")));
    }

    /// Bind `styled` of a compat package, whose function parts read the theme
    /// of its `ThemeProvider`
    fn compat_styled(&mut self, symbol: Option<SymbolId>) {
        self.styled.extend(symbol);
        self.compat.extend(symbol);
    }

    /// Where a specifier of a compat package goes on React Native: `styled`
    /// and `css` compile as Devup UI's, the theme API is the runtime's, and
    /// the rest stays on the package
    fn compat_specifier(
        &mut self,
        specifier: &ImportSpecifier<'_>,
        package: &str,
        target: &NativeTarget,
    ) -> Destination {
        if specifier.import_kind.is_type() {
            return Destination::Kept;
        }
        let name = imported_name(&specifier.imported);
        let symbol = specifier.local.symbol_id.get();
        match name.as_str() {
            "styled" => self.compat_styled(symbol),
            "css" => {
                self.css.extend(symbol);
                self.compat.extend(symbol);
            }
            "ThemeProvider" | "useTheme" | "withTheme" if !target.runtime => {
                self.errors.push((
                    specifier.span.start,
                    compat_theme_problem(&name).import(&name, package),
                ));
                return Destination::Kept;
            }
            "ThemeProvider" | "withTheme" => {
                return Destination::Runtime(renamed(&name, specifier.local.name.to_string()));
            }
            "useTheme" => {
                return Destination::Runtime(renamed(
                    "useStyledTheme",
                    specifier.local.name.to_string(),
                ));
            }
            _ => return Destination::Kept,
        }
        Destination::Dropped
    }

    /// Where a specifier goes on React Native
    fn specifier(
        &mut self,
        specifier: &ImportSpecifier<'_>,
        package: &str,
        target: &NativeTarget,
    ) -> Destination {
        if specifier.import_kind.is_type() {
            return Destination::Kept;
        }
        let name = imported_name(&specifier.imported);
        if let Some(problem) = unsupported(&name, target) {
            self.errors
                .push((specifier.span.start, problem.import(&name, package)));
            return Destination::Kept;
        }
        let local = specifier.local.name.to_string();
        let bindings = match name.as_str() {
            "css" => Some(&mut self.css),
            "styled" => Some(&mut self.styled),
            "keyframes" => Some(&mut self.keyframes),
            _ => None,
        };
        if let Some(bindings) = bindings {
            bindings.extend(specifier.local.symbol_id.get());
            return Destination::Dropped;
        }
        if is_theme_api(&name) {
            return Destination::Runtime(renamed(&name, local));
        }
        let Ok(kind) = ExportVariableKind::try_from(name) else {
            return Destination::Kept;
        };
        let native = native_component(&kind);
        if let Some(symbol) = specifier.local.symbol_id.get() {
            self.components.insert(symbol, (kind, local.clone()));
        }
        Destination::Native(renamed(native, local))
    }
}

/// The specifier importing `name` under `local`
fn renamed(name: &str, local: String) -> String {
    if name == local {
        local
    } else {
        format!("{name} as {local}")
    }
}

/// The name a specifier imports
fn imported_name(name: &ModuleExportName<'_>) -> String {
    name.name().to_string()
}

/// The code of a specifier kept on the package
fn specifier_code(specifier: &ImportSpecifier<'_>, code: &str) -> String {
    specifier.span.source_text(code).to_string()
}
