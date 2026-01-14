//! Import alias transformation
//!
//! Transforms imports from aliased packages to the target package.
//!
//! Examples:
//! - `import styled from '@emotion/styled'` → `import { styled } from '@devup-ui/react'`
//! - `import styledA from '@emotion/styled'` → `import { styled as styledA } from '@devup-ui/react'`
//! - `import { style } from '@vanilla-extract/css'` → `import { style } from '@devup-ui/react'`

use crate::ImportAlias;
use oxc_allocator::Allocator;
use oxc_ast::ast::ImportDeclarationSpecifier;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::HashMap;

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
pub fn transform_import_aliases(
    code: &str,
    filename: &str,
    package: &str,
    import_aliases: &HashMap<String, ImportAlias>,
) -> String {
    // Quick check: if no aliases match, return original code
    if import_aliases.is_empty() || !import_aliases.keys().any(|alias| code.contains(alias)) {
        return code.to_string();
    }

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();

    // Parse the code
    let parser_ret = Parser::new(&allocator, code, source_type).parse();
    let program = parser_ret.program;

    // Collect import transformations
    let mut transformations: Vec<(usize, usize, String)> = Vec::new();

    for stmt in &program.body {
        if let oxc_ast::ast::Statement::ImportDeclaration(import_decl) = stmt {
            let source_value = import_decl.source.value.as_str();

            if let Some(alias) = import_aliases.get(source_value) {
                let span = import_decl.span;
                let new_import = generate_transformed_import(import_decl, alias, package);
                transformations.push((span.start as usize, span.end as usize, new_import));
            }
        }
    }

    // Apply transformations in reverse order to preserve positions
    if transformations.is_empty() {
        return code.to_string();
    }

    let mut result = code.to_string();
    for (start, end, replacement) in transformations.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }

    result
}

/// Generate the transformed import statement
fn generate_transformed_import(
    import_decl: &oxc_ast::ast::ImportDeclaration,
    alias: &ImportAlias,
    package: &str,
) -> String {
    let specifiers = match &import_decl.specifiers {
        Some(specs) => specs,
        None => return format!("import '{}';", package),
    };

    match alias {
        ImportAlias::DefaultToNamed(named_export) => {
            // Transform: `import foo from 'pkg'` → `import { named as foo } from 'target'`
            let mut import_parts = Vec::new();

            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(default_spec) => {
                        let local_name = default_spec.local.name.as_str();
                        if local_name == named_export {
                            // Same name: `import { styled } from 'pkg'`
                            import_parts.push(named_export.clone());
                        } else {
                            // Different name: `import { styled as foo } from 'pkg'`
                            import_parts.push(format!("{} as {}", named_export, local_name));
                        }
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                        // Keep named imports
                        let imported = spec.imported.to_string();
                        let local = spec.local.name.as_str();
                        if imported == local {
                            import_parts.push(imported);
                        } else {
                            import_parts.push(format!("{} as {}", imported, local));
                        }
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns_spec) => {
                        // Namespace imports are kept but shouldn't really happen for CSS-in-JS
                        return format!(
                            "import * as {} from '{}';",
                            ns_spec.local.name.as_str(),
                            package
                        );
                    }
                }
            }

            format!(
                "import {{ {} }} from '{}';",
                import_parts.join(", "),
                package
            )
        }
        ImportAlias::NamedToNamed => {
            // Just change the source, keep specifiers as-is
            // `import { style } from 'pkg'` → `import { style } from 'target'`
            let mut import_parts = Vec::new();

            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(default_spec) => {
                        // Default import in NamedToNamed mode - just keep it
                        // (shouldn't happen for @vanilla-extract/css)
                        import_parts
                            .push(format!("default as {}", default_spec.local.name.as_str()));
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                        let imported = spec.imported.to_string();
                        let local = spec.local.name.as_str();
                        if imported == local {
                            import_parts.push(imported);
                        } else {
                            import_parts.push(format!("{} as {}", imported, local));
                        }
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns_spec) => {
                        return format!(
                            "import * as {} from '{}';",
                            ns_spec.local.name.as_str(),
                            package
                        );
                    }
                }
            }

            format!(
                "import {{ {} }} from '{}';",
                import_parts.join(", "),
                package
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

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
            r#"import styled from '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_different_name() {
        assert_snapshot!(transform_import_aliases(
            r#"import styledA from '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_named_to_named() {
        assert_snapshot!(transform_import_aliases(
            r#"import { style, globalStyle } from '@vanilla-extract/css'"#,
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_no_matching_alias() {
        assert_snapshot!(transform_import_aliases(
            r#"import { useState } from 'react'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_empty_aliases() {
        assert_snapshot!(transform_import_aliases(
            r#"import styled from '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &HashMap::new()
        ));
    }

    #[test]
    fn test_styled_components() {
        assert_snapshot!(transform_import_aliases(
            r#"import styled from 'styled-components'"#,
            "test.tsx",
            "@devup-ui/react",
            &styled_components_alias()
        ));
    }

    #[test]
    fn test_css_ts_file_vanilla_extract() {
        assert_snapshot!(transform_import_aliases(
            r#"import { style } from '@vanilla-extract/css'
export const container = style({ background: 'red' })"#,
            "styles.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_multiple_imports_same_file() {
        assert_snapshot!(transform_import_aliases(
            r#"import styled from '@emotion/styled'
import { style } from '@vanilla-extract/css'
import { useState } from 'react'"#,
            "test.tsx",
            "@devup-ui/react",
            &combined_aliases()
        ));
    }

    #[test]
    fn test_preserves_code_after_import() {
        assert_snapshot!(transform_import_aliases(
            r#"import { style } from '@vanilla-extract/css'

export const button = style({
    background: 'blue',
    padding: '8px',
});"#,
            "test.css.ts",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_import_with_alias() {
        assert_snapshot!(transform_import_aliases(
            r#"import { style as myStyle } from '@vanilla-extract/css'"#,
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_side_effect_import_no_specifiers() {
        assert_snapshot!(transform_import_aliases(
            r#"import '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_alias_in_comment_not_transformed() {
        assert_snapshot!(transform_import_aliases(
            r#"// This uses @emotion/styled but doesn't import it
const x = 1;"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_with_additional_named_imports() {
        assert_snapshot!(transform_import_aliases(
            r#"import styled, { css, keyframes } from '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_with_aliased_named_import() {
        assert_snapshot!(transform_import_aliases(
            r#"import styled, { css as emotionCss } from '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_default_to_named_namespace_import() {
        assert_snapshot!(transform_import_aliases(
            r#"import * as Emotion from '@emotion/styled'"#,
            "test.tsx",
            "@devup-ui/react",
            &emotion_alias()
        ));
    }

    #[test]
    fn test_named_to_named_with_default_specifier() {
        assert_snapshot!(transform_import_aliases(
            r#"import vanillaDefault, { style } from '@vanilla-extract/css'"#,
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }

    #[test]
    fn test_named_to_named_namespace_import() {
        assert_snapshot!(transform_import_aliases(
            r#"import * as VE from '@vanilla-extract/css'"#,
            "test.tsx",
            "@devup-ui/react",
            &vanilla_extract_alias()
        ));
    }
}
