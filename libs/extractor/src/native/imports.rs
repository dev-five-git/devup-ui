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
use rustc_hash::FxHashMap;

use super::errors::{Problem, RuntimeFeature};
use super::{NativeTarget, REACT_NATIVE, Replacement};
use crate::component::ExportVariableKind;

/// The Devup UI components a file imports, and how its imports change
#[derive(Default)]
pub(super) struct Imports {
    /// Each component's binding: its kind and local name
    pub components: FxHashMap<SymbolId, (ExportVariableKind, String)>,
    pub replacements: Vec<Replacement>,
    pub errors: Vec<(u32, String)>,
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

/// Why an export of the package cannot be imported on React Native, or `None`
/// for one that can (a component, or a name only types read)
fn unsupported(name: &str, target: NativeTarget) -> Option<Problem> {
    match name {
        "Grid" => Some(Problem::new(
            "React Native has no grid layout",
            "lay the items out with `Flex` and `flexWrap`",
        )),
        "css" | "styled" => Some(Problem::new(
            format!("`{name}` is not compiled for React Native yet"),
            "write the styles as props of a Devup UI component",
        )),
        "useTheme" | "getTheme" | "setTheme" => Some(RuntimeFeature::Theme.problem(target)),
        "ThemeScript" | "initTheme" | "globalCss" => Some(Problem::new(
            "React Native has no document and no global stylesheet",
            "remove it from the React Native build",
        )),
        "keyframes" => Some(Problem::new(
            "animations need react-native-reanimated, which is not supported yet",
            "animate with React Native's `Animated` or react-native-reanimated directly",
        )),
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
    target: NativeTarget,
) -> Imports {
    let mut imports = Imports::default();
    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(declaration) => {
                if declaration.source.value == package && !declaration.import_kind.is_type() {
                    imports.declaration(declaration, code, package, target);
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
        package: &str,
        target: NativeTarget,
    ) {
        let mut natives = Vec::new();
        let mut kept = Vec::new();
        for specifier in declaration.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    match self.specifier(specifier, package, target) {
                        Some(native) => natives.push(native),
                        None => kept.push(specifier_code(specifier, code)),
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
        if natives.is_empty() {
            return;
        }
        let mut text = format!(
            "import {{ {} }} from \"{REACT_NATIVE}\";",
            natives.join(", ")
        );
        if !kept.is_empty() {
            text = format!(
                "{text}\nimport {{ {} }} from {};",
                kept.join(", "),
                declaration.source.span.source_text(code)
            );
        }
        let span = declaration.span;
        self.replacements
            .push((span.start as usize, span.end as usize, text));
    }

    /// The React Native import a specifier becomes, or `None` to keep it
    fn specifier(
        &mut self,
        specifier: &ImportSpecifier<'_>,
        package: &str,
        target: NativeTarget,
    ) -> Option<String> {
        if specifier.import_kind.is_type() {
            return None;
        }
        let name = imported_name(&specifier.imported);
        if let Some(problem) = unsupported(&name, target) {
            self.errors
                .push((specifier.span.start, problem.import(&name, package)));
            return None;
        }
        let kind = ExportVariableKind::try_from(name).ok()?;
        let local = specifier.local.name.to_string();
        let native = native_component(&kind);
        if let Some(symbol) = specifier.local.symbol_id.get() {
            self.components.insert(symbol, (kind, local.clone()));
        }
        Some(if native == local {
            local
        } else {
            format!("{native} as {local}")
        })
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
