use oxc_span::SourceType;

/// The language of compiler output, independent of its original filename.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExtractSourceType {
    /// JavaScript with optional JSX, never TypeScript or raw Markdown.
    CompiledMdx,
}

pub(crate) type ModuleCacheKey = (String, String, Option<ExtractSourceType>);
pub(crate) type PreparedFileKey = (String, Option<ExtractSourceType>);

/// Parse a supplied source-type literal without accepting unknown modes.
pub fn parse_source_type(value: Option<&str>) -> Result<Option<ExtractSourceType>, String> {
    match value {
        None => Ok(None),
        Some("compiled-mdx") => Ok(Some(ExtractSourceType::CompiledMdx)),
        Some(value) => Err(format!(
            "source type cannot use `{value}` at build time: expected `compiled-mdx`"
        )),
    }
}

pub(crate) fn parser_type(
    filename: &str,
    mode: Option<ExtractSourceType>,
) -> Result<SourceType, String> {
    match mode {
        Some(ExtractSourceType::CompiledMdx) => Ok(SourceType::jsx()),
        None => crate::parser_source_type(filename),
    }
}

pub(crate) fn validate_module(module: &crate::ResolvedModule) -> Result<(), String> {
    let source_type = parser_type(&module.path, module.source_type)?;
    if module.source_type.is_some() || crate::is_compiled_mdx(&module.path) {
        let allocator = oxc_allocator::Allocator::default();
        if let Err(errors) = crate::parsing::parse_source(&allocator, &module.code, source_type) {
            return Err(crate::located_errors(
                &module.path,
                &module.code,
                &[],
                crate::parsing::syntax_errors(errors, true),
                source_type,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modes_when_optional_literal_is_supplied() {
        assert_eq!(parse_source_type(None), Ok(None));
        assert_eq!(
            parse_source_type(Some("compiled-mdx")),
            Ok(Some(ExtractSourceType::CompiledMdx))
        );
        for value in ["", "tsx", "invalid"] {
            assert!(
                parse_source_type(Some(value))
                    .is_err_and(|error| error.contains("expected `compiled-mdx`"))
            );
        }
    }

    #[test]
    fn overrides_language_when_real_extension_is_custom_or_typescript() {
        for filename in ["page.mdown", "page.ts"] {
            assert_eq!(
                parser_type(filename, Some(ExtractSourceType::CompiledMdx)),
                Ok(SourceType::jsx())
            );
        }
        assert_eq!(
            parser_type("page.ts", None),
            SourceType::from_path("page.ts").map_err(|error| error.to_string())
        );
        assert!(
            parser_type("page.mdown", None)
                .is_err_and(|error| error.starts_with("page.mdown:1:1:"))
        );
    }

    #[test]
    fn retains_exports_when_strip_cache_changes_language() -> Result<(), String> {
        let code = "export function view() { return <div/>; } export const PRIMARY = 'red';";
        let filename = "mode-cache.ts";
        let ordinary = crate::vanilla_extract::strip_typescript_with_type(code, filename, None)?;
        let compiled = crate::vanilla_extract::strip_typescript_with_type(
            code,
            filename,
            Some(ExtractSourceType::CompiledMdx),
        )?;
        assert!(compiled.contains("PRIMARY"));
        assert_ne!(compiled, ordinary);
        assert_eq!(
            crate::vanilla_extract::strip_typescript_with_type(
                code,
                filename,
                Some(ExtractSourceType::CompiledMdx)
            )?,
            compiled
        );
        assert!(
            crate::vanilla_extract::strip_typescript_with_type(code, "bad.custom", None).is_err()
        );
        Ok(())
    }

    #[test]
    fn rejects_markdown_when_resolved_module_claims_compiled_output() {
        let module = crate::ResolvedModule {
            path: "page.mdown".to_string(),
            code: "# Heading".to_string(),
            source_type: Some(ExtractSourceType::CompiledMdx),
        };
        assert!(validate_module(&module).is_err_and(
            |error| error.starts_with("page.mdown:1:") && error.contains("raw or invalid MDX")
        ));
    }

    #[test]
    fn reports_all_recoverable_errors_when_compiler_output_has_no_styling_import() {
        // Given: multiple recoverable declarations in explicitly compiled output.
        let source = "const first;\nconst second;";
        // When: the public entry validates a custom filename before preprocessing.
        let error = crate::extract_with_source_type(
            "page.mdown",
            source,
            crate::ExtractOption::default(),
            false,
            None,
            Some(ExtractSourceType::CompiledMdx),
        )
        .err()
        .unwrap_or_else(|| panic!("invalid compiled output was accepted"))
        .to_string();
        // Then: every diagnostic keeps its original source line and MDX context.
        let errors: Vec<_> = error.lines().collect();
        assert_eq!(errors.len(), 2, "{error}");
        assert!(errors[0].starts_with("page.mdown:1:"), "{error}");
        assert!(errors[1].starts_with("page.mdown:2:"), "{error}");
        assert!(
            errors
                .iter()
                .all(|error| error.contains("raw or invalid MDX"))
        );
    }
}
