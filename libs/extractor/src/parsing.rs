use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::Program;
use oxc_parser::{ParseOptions, Parser, ParserReturn};
use oxc_span::{SourceType, UnknownExtension};
#[cfg(test)]
use std::error::Error;

pub(super) mod goal_policy;

#[cfg(test)]
mod hybrid_tests;

/// Accept JavaScript JSX without changing the path's TypeScript or module dialect.
pub(super) fn source_type(filename: &str) -> Result<SourceType, UnknownExtension> {
    SourceType::from_path(filename)
        .map(|source_type| source_type.with_jsx(source_type.is_javascript()))
}

fn parse_physical<'a>(
    allocator: &'a Allocator,
    code: &'a str,
    source_type: SourceType,
) -> ParserReturn<'a> {
    Parser::new(allocator, code, source_type)
        .with_options(ParseOptions {
            parse_regular_expression: true,
            ..ParseOptions::default()
        })
        .parse()
}

/// Validate physical grammar while keeping validation's semantic IDs off the returned AST.
pub(super) fn checked<'a>(
    allocator: &'a Allocator,
    code: &'a str,
    source_type: SourceType,
) -> Result<Program<'a>, Vec<(u32, String)>> {
    let parsed = parse_physical(allocator, code, source_type);
    let parser_error = parsed
        .diagnostics
        .errors()
        .filter(|error| {
            !goal_policy::accepts_declaration_goal(&parsed, &error.to_string(), &error.labels)
        })
        .min_by_key(|error| {
            goal_policy::is_declaration_goal(&parsed, &error.to_string(), &error.labels)
        });
    let validation_allocator = Allocator::default();
    let semantic_diagnostics = if parser_error.is_none() && !parsed.fatal_error {
        let validation_program = parsed.program.clone_in(&validation_allocator);
        oxc_semantic::SemanticBuilder::new()
            .with_check_syntax_error(true)
            .build(&validation_program)
            .diagnostics
    } else {
        Default::default()
    };
    if let Some(error) = parser_error.or_else(|| semantic_diagnostics.errors().next()) {
        let offset = error
            .labels
            .iter()
            .find(|label| label.primary())
            .or_else(|| error.labels.first())
            .map_or(0, oxc_span::LabeledSpan::offset);
        return Err(vec![(
            offset,
            format!(
                "Cannot parse source: {}. Fix: correct the syntax at this location",
                error.to_string().trim_end_matches('.')
            ),
        )]);
    }
    Ok(parsed.program)
}

/// Parse source, reporting syntax errors at their primary source locations.
#[cfg(test)]
pub(super) fn parse<'a>(
    allocator: &'a Allocator,
    filename: &str,
    code: &'a str,
) -> Result<Program<'a>, Box<dyn Error>> {
    let parsed = parse_physical(allocator, code, source_type(filename)?);
    let mut errors: Vec<_> = parsed
        .diagnostics
        .errors()
        .filter_map(|error| {
            let message = error.to_string();
            if goal_policy::accepts_declaration_goal(&parsed, &message, &error.labels) {
                return None;
            }
            let offset = error
                .labels
                .iter()
                .find(|label| label.primary())
                .or_else(|| error.labels.first())
                .map_or(0, oxc_span::LabeledSpan::offset);
            Some((offset, message))
        })
        .collect();
    if !errors.is_empty() {
        errors.sort_unstable();
        errors.dedup();
        return Err(errors
            .into_iter()
            .map(|(offset, message)| {
                format!(
                    "{}: {message}",
                    crate::locate(filename, code, offset as usize)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into());
    }
    Ok(parsed.program)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{parse, source_type};
    use crate::extract_style::extract_style_value::ExtractStyleValue;
    use crate::{
        ExtractOption, ImportAlias, ResolvedModule, extract, extract_with_modules, has_devup_ui,
    };
    use css::{class_map::reset_class_map, file_map::reset_file_map};
    use oxc_allocator::Allocator;
    use oxc_span::SourceType;
    use rstest::rstest;
    use serial_test::serial;
    use std::collections::HashMap;

    #[rstest]
    #[case("page.js", true)]
    #[case("page.mjs", true)]
    #[case("page.cjs", true)]
    #[case("page.jsx", true)]
    #[case("page.ts", false)]
    #[case("page.mts", false)]
    #[case("page.cts", false)]
    #[case("page.tsx", true)]
    #[case("page.d.ts", false)]
    #[case("page.d.mts", false)]
    #[case("page.d.cts", false)]
    fn preserves_path_dialect(#[case] filename: &str, #[case] jsx: bool) {
        // Given the parser's original path-derived language and module kind.
        let original = SourceType::from_path(filename).unwrap();
        // When JavaScript JSX permission is applied.
        let actual = source_type(filename).unwrap();
        // Then no other dialect metadata changes.
        assert_eq!(actual.is_jsx(), jsx);
        assert_eq!(actual.is_typescript(), original.is_typescript());
        assert_eq!(
            actual.is_typescript_definition(),
            original.is_typescript_definition()
        );
        assert_eq!(actual.module_kind(), original.module_kind());
        assert_eq!(actual.extension(), original.extension());
    }

    #[rstest]
    #[case(
        "page.ts",
        "const value = <number>1; const identity = <T>(value: T): T => value;"
    )]
    #[case(
        "page.mts",
        "const value = 1 as number; const identity = <T,>(value: T): T => value;"
    )]
    #[case(
        "page.cts",
        "const value = 1 as number; const identity = <T,>(value: T): T => value;"
    )]
    fn preserves_typescript_angle_syntax(#[case] filename: &str, #[case] source: &str) {
        // Given syntax that TSX cannot interpret as TypeScript assertions.
        let allocator = Allocator::default();
        // When parsing a non-TSX TypeScript extension.
        let program = parse(&allocator, filename, source).unwrap();
        // Then both declarations remain TypeScript, not JSX.
        assert_eq!(program.body.len(), 2);
        assert!(!program.source_type.is_jsx());
    }

    #[rstest]
    #[case("src/app/page.js")]
    #[case("src/app/page.mjs")]
    #[case("src/app/page.cjs")]
    #[serial]
    fn extracts_jsx_in_javascript(#[case] filename: &str) {
        // Given a JavaScript module accepted by React toolchains.
        reset_class_map();
        reset_file_map();
        let source = "import { Box } from '@devup-ui/react';\nexport default function Page() { return <Box bg=\"maroon\" p=\"13px\">proof</Box>; }";

        // When extraction uses the physical JavaScript filename.
        let output = extract(filename, source, ExtractOption::default()).unwrap();

        // Then styling is compiled away and package detection sees the import.
        assert!(output.code.contains("<div className="));
        assert!(!output.code.contains("<Box"));
        assert_eq!(output.styles.len(), 2);
        assert!(has_devup_ui(filename, source, "@devup-ui/react"));
    }

    #[rstest]
    #[case(
        "import { Box } from '@devup-ui/react';\nconst view = <Box",
        "src/app/page.js:2:18:"
    )]
    #[case(
        "import { css } from '@devup-ui/react';\nconst broken = ;",
        "src/app/page.js:2:16:"
    )]
    #[serial]
    fn locates_parse_errors(#[case] source: &str, #[case] location: &str) {
        // Given invalid source with an imported extraction API.
        // When the parser rejects it, extraction returns a build diagnostic.
        let error = extract("src/app/page.js", source, ExtractOption::default())
            .unwrap_err()
            .to_string();

        // Then the diagnostic identifies the source rather than a parser panic.
        assert!(error.starts_with(location), "{error}");
        assert!(!error.contains("panicked"));
    }

    #[rstest]
    #[case("js")]
    #[case("mjs")]
    #[case("cjs")]
    #[serial]
    fn evaluates_emotion_values_beside_jsx(#[case] extension: &str) {
        // Given computed Emotion values and a JSX sibling in a JS module.
        reset_class_map();
        reset_file_map();
        let source = "import { css } from '@emotion/css'; function twice(value) { return value * 2; } export const style = css({ width: twice(2) }); export const view = <div className={style} />;";
        let option = ExtractOption {
            import_aliases: HashMap::from([(
                "@emotion/css".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        };
        // When the alias, evaluator and computed-number reparse run.
        let output = extract(&format!("page.{extension}"), source, option).unwrap();
        // Then the computed number keeps Emotion's pixel semantics.
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "width" && style.value == "4px")));
        assert!(output.code.contains("<div className={style}"));
    }

    #[rstest]
    #[case("js", "export const color = 'teal'; export const view = <div />;")]
    #[case("mjs", "export const color = 'teal'; export const view = <div />;")]
    #[case("cjs", "exports.color = 'teal'; exports.view = <div />;")]
    #[serial]
    fn reads_constants_from_javascript_with_jsx(
        #[case] extension: &str,
        #[case] module: &'static str,
    ) {
        // Given a dependency with a static constant and unrelated JSX.
        reset_class_map();
        reset_file_map();
        let path = format!("tokens.{extension}");
        let dependency = path.clone();
        let resolver = move |specifier: &str, _: &str| {
            (specifier == "./tokens").then(|| ResolvedModule {
                path: path.clone(),
                code: module.to_string(),
            })
        };
        // When a style reads the constant without executing the JSX export.
        let output = extract_with_modules("page.jsx", "import { Box } from '@devup-ui/react'; import { color } from './tokens'; export const view = <Box color={color} />;", ExtractOption::default(), false, &resolver).unwrap();
        // Then extraction emits the known color and tracks the dependency.
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property == "color" && style.value == "teal")));
        assert_eq!(output.dependencies, [dependency]);
    }

    #[rstest]
    #[case("page.js")]
    #[case("page.mjs")]
    #[case("page.cjs")]
    #[serial]
    fn preserves_jsx_in_namespace_only_output(#[case] filename: &str) {
        // Given an unused namespace import beside unrelated JSX.
        let source = "import * as E from '@emotion/css'; export const view = <div />;";
        let option = ExtractOption {
            import_aliases: HashMap::from([(
                "@emotion/css".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        };
        // When namespace-only output takes the no-style extraction path.
        let output = extract(filename, source, option).unwrap();
        // Then unrelated JSX and its original source map survive.
        assert!(output.code.contains("<div />"));
        assert_eq!(output.styles.len(), 0);
        let map: serde_json::Value = serde_json::from_str(&output.map.unwrap()).unwrap();
        assert_eq!(map["sourcesContent"][0].as_str(), Some(source));
    }

    #[test]
    #[serial]
    fn rejects_recoverable_syntax_before_alias_rewriting() {
        // Given a recoverable syntax error in an aliased module.
        let source = "import * as E from '@emotion/css';\nE.hydrate([]); const broken;";
        let option = ExtractOption {
            import_aliases: HashMap::from([(
                "@emotion/css".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        };
        // When a preprocessing pass would otherwise hide the invalid input.
        let error = extract("page.js", source, option).unwrap_err().to_string();
        // Then the syntax diagnostic refers to the original physical source.
        assert!(error.starts_with("page.js:2:"), "{error}");
        assert!(error.contains("Missing initializer"), "{error}");
    }

    #[rstest]
    #[case("wrong")]
    #[case("className")]
    #[case("styleOrder")]
    #[case("styleVars")]
    #[serial]
    fn rejects_empty_jsx_attributes(#[case] attribute: &str) {
        // Given an empty attribute expression, which is invalid JSX.
        let source = format!(
            "import {{ Box }} from '@devup-ui/react';\nconst view = <Box {attribute}={{}} />;"
        );
        // When extraction encounters a recoverable parser diagnostic.
        let error = extract("page.js", &source, ExtractOption::default())
            .unwrap_err()
            .to_string();
        // Then invalid source is not silently emitted or compiled as an absent prop.
        assert!(error.starts_with("page.js:2:"), "{error}");
        assert!(error.contains("non-empty 'expression'"), "{error}");
    }
}
