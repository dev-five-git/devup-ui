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

    #[test]
    fn test_default_to_named_same_name() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        let code = r#"import styled from '@emotion/styled'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        assert!(
            result.contains("import { styled } from '@devup-ui/react'"),
            "Expected named import, got: {}",
            result
        );
    }

    #[test]
    fn test_default_to_named_different_name() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        let code = r#"import styledA from '@emotion/styled'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        assert!(
            result.contains("styled as styledA"),
            "Expected 'styled as styledA', got: {}",
            result
        );
        assert!(
            result.contains("@devup-ui/react"),
            "Expected '@devup-ui/react', got: {}",
            result
        );
    }

    #[test]
    fn test_named_to_named() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        let code = r#"import { style, globalStyle } from '@vanilla-extract/css'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        assert!(
            result.contains("style"),
            "Expected 'style', got: {}",
            result
        );
        assert!(
            result.contains("globalStyle"),
            "Expected 'globalStyle', got: {}",
            result
        );
        assert!(
            result.contains("@devup-ui/react"),
            "Expected '@devup-ui/react', got: {}",
            result
        );
    }

    #[test]
    fn test_no_matching_alias() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        let code = r#"import { useState } from 'react'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        // Should be unchanged
        assert!(
            result.contains("react"),
            "Expected 'react' import unchanged, got: {}",
            result
        );
        assert!(
            !result.contains("@devup-ui/react"),
            "Should not contain '@devup-ui/react', got: {}",
            result
        );
    }

    #[test]
    fn test_empty_aliases() {
        let aliases = HashMap::new();
        let code = r#"import styled from '@emotion/styled'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        // Should return original code
        assert_eq!(result, code);
    }

    #[test]
    fn test_styled_components() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "styled-components".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        let code = r#"import styled from 'styled-components'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        assert!(
            result.contains("import { styled } from '@devup-ui/react'"),
            "Expected named import, got: {}",
            result
        );
    }

    #[test]
    fn test_css_ts_file_vanilla_extract() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        let code = r#"import { style } from '@vanilla-extract/css'
export const container = style({ background: 'red' })"#;
        let result = transform_import_aliases(code, "styles.css.ts", "@devup-ui/react", &aliases);

        assert!(
            result.contains("@devup-ui/react"),
            "Expected '@devup-ui/react', got: {}",
            result
        );
        assert!(
            result.contains("style"),
            "Expected 'style' import, got: {}",
            result
        );
    }

    #[test]
    fn test_multiple_imports_same_file() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        let code = r#"import styled from '@emotion/styled'
import { style } from '@vanilla-extract/css'
import { useState } from 'react'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        // Both aliased imports should be transformed
        assert!(
            result.matches("@devup-ui/react").count() == 2,
            "Expected 2 @devup-ui/react imports, got: {}",
            result
        );
        // React import should be unchanged
        assert!(
            result.contains("react"),
            "Expected 'react' import, got: {}",
            result
        );
    }

    #[test]
    fn test_preserves_code_after_import() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        let code = r#"import { style } from '@vanilla-extract/css'

export const button = style({
    background: 'blue',
    padding: '8px',
});"#;
        let result = transform_import_aliases(code, "test.css.ts", "@devup-ui/react", &aliases);

        // Import should be transformed
        assert!(
            result.contains("@devup-ui/react"),
            "Expected '@devup-ui/react', got: {}",
            result
        );
        // Rest of the code should be preserved
        assert!(
            result.contains("export const button = style"),
            "Expected style call preserved, got: {}",
            result
        );
        assert!(
            result.contains("background: 'blue'"),
            "Expected styles preserved, got: {}",
            result
        );
    }

    #[test]
    fn test_named_import_with_alias() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        let code = r#"import { style as myStyle } from '@vanilla-extract/css'"#;
        let result = transform_import_aliases(code, "test.tsx", "@devup-ui/react", &aliases);

        assert!(
            result.contains("style as myStyle"),
            "Expected 'style as myStyle', got: {}",
            result
        );
        assert!(
            result.contains("@devup-ui/react"),
            "Expected '@devup-ui/react', got: {}",
            result
        );
    }
}
