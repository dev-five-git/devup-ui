//! Import alias transformation
//!
//! Transforms imports from aliased packages to the target package.
//!
//! Examples:
//! - `import styled from '@emotion/styled'` → `import { styled } from '@devup-ui/react'`
//! - `import styledA from '@emotion/styled'` → `import { styled as styledA } from '@devup-ui/react'`
//! - `import { style } from '@vanilla-extract/css'` → `import { style } from '@devup-ui/react'`

use crate::ImportAlias;
use crate::utils::is_vanilla_extract_file;
use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclarationSpecifier, ModuleExportName};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::borrow::Cow;
use std::collections::HashMap;

/// Map an aliased package's export onto the `@devup-ui/react` export that implements the
/// same behaviour, so the extractor consumes the call and drops the import entirely — no
/// dependency on either package survives.
///
/// `None` means the name has no devup-ui counterpart. Redirecting it anyway would produce
/// an ESM "does not provide an export" error, so the specifier stays on its own package
/// and the source library remains a real dependency.
/// Where a redirected specifier lands.
///
/// `Main` names are genuine Devup UI APIs. `Compat` names only exist to absorb another
/// library, so they live in the `<package>/compat` entry and never widen what a project
/// using Devup UI directly sees — which also lets them keep their original spelling
/// (`useTheme` there cannot collide with Devup UI's own `useTheme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DevupTarget<'n> {
    Main(&'n str),
    Compat(&'n str),
}

fn devup_equivalent(source: &str, imported: &str) -> Option<DevupTarget<'static>> {
    match (source, imported) {
        // `style({...})` and `css({...})` both hand back a class name for a style object,
        // and `globalStyle(selector, rules)` is `globalCss` with the selector split out.
        ("@vanilla-extract/css", "style") | (_, "css") => Some(DevupTarget::Main("css")),
        ("@vanilla-extract/css", "globalStyle") => Some(DevupTarget::Main("globalCss")),
        (_, "keyframes") => Some(DevupTarget::Main("keyframes")),
        (_, "styled") => Some(DevupTarget::Main("styled")),
        (_, "createGlobalStyle") => Some(DevupTarget::Compat("createGlobalStyle")),
        (_, "Global") => Some(DevupTarget::Compat("Global")),
        (_, "ThemeProvider") => Some(DevupTarget::Compat("ThemeProvider")),
        (_, "ServerStyleSheet") => Some(DevupTarget::Compat("ServerStyleSheet")),
        (_, "StyleSheetManager") => Some(DevupTarget::Compat("StyleSheetManager")),
        (_, "isStyledComponent") => Some(DevupTarget::Compat("isStyledComponent")),
        (_, "withTheme") => Some(DevupTarget::Compat("withTheme")),
        (_, "useTheme") => Some(DevupTarget::Compat("useTheme")),
        _ => None,
    }
}

/// Transform source code by rewriting aliased imports to the target package
///
/// # Arguments
/// * `code` - The source code to transform
/// * `filename` - The filename (used for source type detection)
/// * `package` - The target package (e.g., "@devup-ui/react")
/// * `import_aliases` - Map of source package → alias configuration
///
/// # Returns
/// The transformed source code, or the original code if no transformations were needed
pub fn transform_import_aliases<'a>(
    code: &'a str,
    filename: &str,
    package: &str,
    import_aliases: &HashMap<String, ImportAlias>,
) -> Cow<'a, str> {
    // Quick check: if no aliases match, return original code
    if import_aliases.is_empty() || !import_aliases.keys().any(|alias| code.contains(alias)) {
        return Cow::Borrowed(code);
    }

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();

    // Parse the code
    let parser_ret = Parser::new(&allocator, code, source_type).parse();
    let program = parser_ret.program;

    let redirect_every_name = is_vanilla_extract_file(filename);

    // Collect import transformations
    let mut transformations: Vec<(usize, usize, String)> = Vec::new();

    for stmt in &program.body {
        if let oxc_ast::ast::Statement::ImportDeclaration(import_decl) = stmt {
            let source_value = import_decl.source.value.as_str();

            if let Some(alias) = import_aliases.get(source_value) {
                let span = import_decl.span;
                let new_import =
                    generate_transformed_import(import_decl, alias, package, redirect_every_name);
                transformations.push((span.start as usize, span.end as usize, new_import));
            }
        }
    }

    // Apply transformations in reverse order to preserve positions
    if transformations.is_empty() {
        return Cow::Borrowed(code);
    }

    let mut result = code.to_string();
    for (start, end, replacement) in transformations.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }

    Cow::Owned(result)
}

/// Pick the name a specifier should import from the target package, or `None` to leave it
/// on its own package. A vanilla-extract stylesheet bypasses the mapping because
/// `execute_vanilla_extract` destructures its mock namespace by vanilla-extract's own names.
fn redirect_target<'n>(
    source: &str,
    imported: &'n str,
    redirect_every_name: bool,
) -> Option<DevupTarget<'n>> {
    if redirect_every_name {
        Some(DevupTarget::Main(imported))
    } else {
        devup_equivalent(source, imported)
    }
}

fn split_target<'n>(target: DevupTarget<'n>, package: &str) -> (&'n str, String) {
    match target {
        DevupTarget::Main(name) => (name, package.to_string()),
        DevupTarget::Compat(name) => (name, format!("{package}/compat")),
    }
}

fn push_redirect(
    redirected: &mut String,
    compat: &mut String,
    target: DevupTarget<'_>,
    local: &str,
) {
    match target {
        DevupTarget::Main(name) => push_specifier(redirected, name, local),
        DevupTarget::Compat(name) => push_specifier(compat, name, local),
    }
}

fn push_specifier(parts: &mut String, imported: &str, local: &str) {
    if !parts.is_empty() {
        parts.push_str(", ");
    }
    parts.push_str(imported);
    if imported != local {
        parts.push_str(" as ");
        parts.push_str(local);
    }
}

/// Borrow the exported name for the common identifier cases to avoid a per-specifier
/// heap allocation. Only the rare string-literal export name (`import { "x" as y }`)
/// needs an owned `String`, and its `Display` output is quoted.
fn imported_name<'a>(imported: &'a ModuleExportName) -> Cow<'a, str> {
    match imported {
        ModuleExportName::IdentifierName(id) => Cow::Borrowed(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => Cow::Borrowed(id.name.as_str()),
        ModuleExportName::StringLiteral(_) => Cow::Owned(imported.to_string()),
    }
}

/// Generate the transformed import statement
fn generate_transformed_import(
    import_decl: &oxc_ast::ast::ImportDeclaration,
    alias: &ImportAlias,
    package: &str,
    redirect_every_name: bool,
) -> String {
    let source = import_decl.source.value.as_str();
    let specifiers = match &import_decl.specifiers {
        Some(specs) => specs,
        None => return format!("import '{package}';"),
    };

    // Classify specifiers in a single pass: capture the first namespace and the
    // first default specifier (first-seen wins, matching the prior find_map
    // semantics). Named specifiers are still iterated separately below.
    let mut namespace = None;
    let mut default_spec = None;
    for s in specifiers {
        match s {
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                if namespace.is_none() {
                    namespace = Some(ns);
                }
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(ds) => {
                if default_spec.is_none() {
                    default_spec = Some(ds);
                }
            }
            ImportDeclarationSpecifier::ImportSpecifier(_) => {}
        }
    }

    if let Some(ns_spec) = namespace {
        let local = ns_spec.local.name.as_str();
        // A `DefaultToNamed` package exports a single callable, so its namespace binding
        // *is* that value — bind it straight to the devup-ui export and member calls such
        // as `Emotion.div` keep resolving, with no dependency left behind.
        if let ImportAlias::DefaultToNamed(named_export) = alias
            && let Some(target) = redirect_target(source, named_export, redirect_every_name)
        {
            let (devup_name, entry) = split_target(target, package);
            let mut parts = String::new();
            push_specifier(&mut parts, devup_name, local);
            return format!("import {{ {parts} }} from '{entry}';");
        }
        // Otherwise the namespace stands for many named exports whose devup-ui
        // counterparts can be renamed (`style` -> `css`), which a namespace access
        // cannot express. Leave it on its own package rather than break the members.
        let target = if redirect_every_name { package } else { source };
        return format!("import * as {local} from '{target}';");
    }

    let mut redirected = String::new();
    let mut compat = String::new();
    let mut retained = String::new();
    let mut retained_default = None;

    // Handle default specifier first (at most one in valid JS); only its
    // rendering differs between the alias variants.
    if let Some(default_spec) = default_spec {
        let local_name = default_spec.local.name.as_str();
        match alias {
            // `import foo from 'pkg'` → `import { named as foo } from 'target'`
            ImportAlias::DefaultToNamed(named_export) => {
                match redirect_target(source, named_export, redirect_every_name) {
                    Some(target) => push_redirect(&mut redirected, &mut compat, target, local_name),
                    None => retained_default = Some(local_name),
                }
            }
            // `import foo from 'pkg'` → `import { default as foo } from 'target'`
            ImportAlias::NamedToNamed => {
                if redirect_every_name {
                    push_redirect(
                        &mut redirected,
                        &mut compat,
                        DevupTarget::Main("default"),
                        local_name,
                    );
                } else {
                    retained_default = Some(local_name);
                }
            }
        }
    }

    // Handle named specifiers (kept as-is for both variants)
    for specifier in specifiers {
        if let ImportDeclarationSpecifier::ImportSpecifier(spec) = specifier {
            let local = spec.local.name.as_str();
            let imported = imported_name(&spec.imported);
            match redirect_target(source, &imported, redirect_every_name) {
                Some(target) => push_redirect(&mut redirected, &mut compat, target, local),
                None => push_specifier(&mut retained, &imported, local),
            }
        }
    }

    let mut result = String::new();
    for (parts, entry) in [
        (&redirected, package.to_string()),
        (&compat, format!("{package}/compat")),
    ] {
        if parts.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str("import { ");
        result.push_str(parts);
        result.push_str(" } from '");
        result.push_str(&entry);
        result.push_str("';");
    }
    if retained_default.is_some() || !retained.is_empty() {
        eprintln!(
            "[devup-ui] WARNING: '{source}' keeps {} because devup-ui has no equivalent export, so the package stays a runtime dependency.",
            retained_default.map_or_else(|| retained.clone(), ToString::to_string)
        );
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str("import ");
        if let Some(local) = retained_default {
            result.push_str(local);
            if !retained.is_empty() {
                result.push_str(", ");
            }
        }
        if !retained.is_empty() {
            result.push_str("{ ");
            result.push_str(&retained);
            result.push_str(" }");
        }
        result.push_str(" from '");
        result.push_str(source);
        result.push_str("';");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use oxc_ast::builder::AstBuilder;
    use oxc_span::SPAN;

    fn emotion_alias() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases
    }

    fn vanilla_extract_alias() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );
        aliases
    }

    fn styled_components_alias() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "styled-components".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases
    }

    fn combined_aliases() -> HashMap<String, ImportAlias> {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );
        aliases
    }

    #[test]
    fn test_default_to_named_same_name() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_different_name() {
        assert_snapshot!(transform_import_aliases(
            r"import styledA from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_named_to_named() {
        assert_snapshot!(transform_import_aliases(
            r"import { style, globalStyle } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_default_specifier_in_vanilla_extract_stylesheet() {
        assert_snapshot!(transform_import_aliases(
            r"import veDefault from '@vanilla-extract/css'",
            "styles.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_default_and_named_both_retained_on_source() {
        assert_snapshot!(transform_import_aliases(
            r"import veDefault, { styleVariants } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_default_export_without_devup_equivalent_stays_on_source() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "some-lib".to_string(),
            ImportAlias::DefaultToNamed("someUnmappedExport".to_string()),
        );
        assert_snapshot!(transform_import_aliases(
            r"import sheet from 'some-lib'",
            "test.tsx",
            "@devup-ui/react",
            &aliases
        ));
    }

    #[test]
    fn test_namespace_import_of_a_compat_only_default() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "some-lib".to_string(),
            ImportAlias::DefaultToNamed("ThemeProvider".to_string()),
        );
        assert_snapshot!(transform_import_aliases(
            r"import * as Sheet from 'some-lib'",
            "test.tsx",
            "@devup-ui/react",
            &aliases
        ));
    }

    #[test]
    fn test_vanilla_extract_names_map_onto_devup_equivalents() {
        assert_snapshot!(transform_import_aliases(
            r"import { style, globalStyle, styleVariants } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_imports_without_devup_export_stay_on_source() {
        assert_snapshot!(transform_import_aliases(
            r"import styled, { css, keyframes, createGlobalStyle, ThemeProvider } from 'styled-components'",
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias()
        ));
    }

    #[test]
    fn test_no_matching_alias() {
        assert_snapshot!(transform_import_aliases(
            r"import { useState } from 'react'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_empty_aliases() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &HashMap::new()
        ));
    }

    #[test]
    fn test_styled_components() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from 'styled-components'",
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias()
        ));
    }

    #[test]
    fn test_css_ts_file_vanilla_extract() {
        assert_snapshot!(transform_import_aliases(
            r"import { style } from '@vanilla-extract/css'
export const container = style({ background: 'red' })",
            "styles.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_multiple_imports_same_file() {
        assert_snapshot!(transform_import_aliases(
            r"import styled from '@emotion/styled'
import { style } from '@vanilla-extract/css'
import { useState } from 'react'",
            "test.tsx",
            "@devup-ui/react",
            &combined_aliases()
        ));
    }

    #[test]
    fn test_preserves_code_after_import() {
        assert_snapshot!(transform_import_aliases(
            r"import { style } from '@vanilla-extract/css'

export const button = style({
    background: 'blue',
    padding: '8px',
});",
            "test.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_import_with_alias() {
        assert_snapshot!(transform_import_aliases(
            r"import { style as myStyle } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_side_effect_import_no_specifiers() {
        assert_snapshot!(transform_import_aliases(
            r"import '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_alias_in_comment_not_transformed() {
        assert_snapshot!(transform_import_aliases(
            r"// This uses @emotion/styled but doesn't import it
const x = 1;",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_with_additional_named_imports() {
        assert_snapshot!(transform_import_aliases(
            r"import styled, { css, keyframes } from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_with_aliased_named_import() {
        assert_snapshot!(transform_import_aliases(
            r"import styled, { css as emotionCss } from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_namespace_import() {
        assert_snapshot!(transform_import_aliases(
            r"import * as Emotion from '@emotion/styled'",
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_named_to_named_with_default_specifier() {
        assert_snapshot!(transform_import_aliases(
            r"import vanillaDefault, { style } from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_to_named_namespace_import() {
        assert_snapshot!(transform_import_aliases(
            r"import * as VE from '@vanilla-extract/css'",
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_identifier_reference_imported_name_with_and_without_alias() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);

        for (code, local, expected) in [
            (
                "import { imported } from 'source'",
                "imported",
                "import { imported } from '@devup-ui/react';",
            ),
            (
                "import { imported as local } from 'source'",
                "local",
                "import { imported as local } from '@devup-ui/react';",
            ),
        ] {
            let mut parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
            let oxc_ast::ast::Statement::ImportDeclaration(import_decl) =
                &mut parsed.program.body[0]
            else {
                panic!("expected import declaration");
            };
            let Some(specifiers) = import_decl.specifiers.as_mut() else {
                panic!("expected import specifiers");
            };
            let ImportDeclarationSpecifier::ImportSpecifier(spec) = &mut specifiers[0] else {
                panic!("expected import specifier");
            };
            spec.imported = ModuleExportName::new_identifier_reference(SPAN, "imported", &builder);
            assert_eq!(spec.local.name.as_str(), local);
            assert_eq!(
                generate_transformed_import(
                    import_decl,
                    &ImportAlias::NamedToNamed,
                    "@devup-ui/react",
                    true
                ),
                expected
            );
        }
    }

    #[test]
    fn test_string_literal_imported_name_with_and_without_alias() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);

        for (code, expected) in [
            (
                "import { 'imported' as local } from 'source'",
                "import { \"imported\" as local } from '@devup-ui/react';",
            ),
            (
                "import { imported } from 'source'",
                "import { \"imported\" } from '@devup-ui/react';",
            ),
        ] {
            let mut parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
            let oxc_ast::ast::Statement::ImportDeclaration(import_decl) =
                &mut parsed.program.body[0]
            else {
                panic!("expected import declaration");
            };
            let Some(specifiers) = import_decl.specifiers.as_mut() else {
                panic!("expected import specifiers");
            };
            let ImportDeclarationSpecifier::ImportSpecifier(spec) = &mut specifiers[0] else {
                panic!("expected import specifier");
            };
            if spec.local.name == "imported" {
                spec.imported =
                    ModuleExportName::new_string_literal(SPAN, "imported", None, &builder);
                spec.local.name = "\"imported\"".into();
            }
            assert_eq!(
                generate_transformed_import(
                    import_decl,
                    &ImportAlias::NamedToNamed,
                    "@devup-ui/react",
                    true
                ),
                expected
            );
        }
    }
}
