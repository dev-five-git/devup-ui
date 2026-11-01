use crate::{ExtractOption, ExtractStyleValue, extract, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

mod classes;

#[test]
#[serial]
fn imported_constant_when_unused_require_sources_are_surrogates_still_extracts() {
    // Given an imported literal export alongside unused, unrepresentable requires.
    reset_class_map();
    reset_file_map();
    let resolver = |source: &str, _: &str| {
        assert!(
            !source.contains('\u{FFFD}'),
            "lossy module source: {source:?}"
        );
        (source == "./tokens").then(|| crate::ResolvedModule {
            path: "/tokens.js".to_string(),
            code: r"const high = require('\uD800');
const low = require('\uDC00');
export const spacing = 3;"
                .to_string(),
        })
    };
    // When constant collection reads the imported module, not just the root scope.
    let output = extract_with_modules(
        "/app.tsx",
        "import { css } from '@devup-ui/react'; import { spacing } from './tokens'; export const result = css({ p: spacing });",
        ExtractOption::default(),
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then optional opaque sources do not prevent exact static constant consumption.
    assert_eq!(output.dependencies, vec!["/tokens.js"]);
    let styles: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    assert_eq!(styles, vec![("padding", "12px")]);
    assert!(!output.code.contains("css("), "{}", output.code);
    assert!(!output.code.contains("var(--"), "{}", output.code);
}

#[rstest]
#[case(r"'\uD800'")]
#[case(r"'\uDC00'")]
#[serial]
fn optional_module_recognition_preserves_surrogate_sources(#[case] literal: &str) {
    use oxc_ast_visit::{Visit, walk};
    struct Sources(Vec<Vec<u16>>);
    impl<'a> Visit<'a> for Sources {
        fn visit_import_declaration(&mut self, import: &oxc_ast::ast::ImportDeclaration<'a>) {
            if import.source.value.as_str().is_none() {
                self.0.push(import.source.value.encode_utf16().collect());
            }
        }
        fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
            if matches!(&call.callee, Expression::Identifier(name) if name.name == "require")
                && let [oxc_ast::ast::Argument::StringLiteral(source)] = call.arguments.as_slice()
            {
                self.0.push(source.value.encode_utf16().collect());
            }
            walk::walk_call_expression(self, call);
        }
    }
    // Given an unrelated import and require alongside an admitted static calculation.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import unrelated from {literal};\nimport {{ css }} from '@devup-ui/react';\nconst unused = require({literal});\nfunction twice(n) {{ return n * 2; }}\nexport const result = css({{ p: twice(2) }});\nexport {{ unrelated, unused }};"
    );
    // When the real module-aware extraction pipeline scans optional imports.
    let output = extract_with_modules(
        "optional.tsx",
        &source,
        ExtractOption::default(),
        false,
        &|_, _| None,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then the opaque sources stay authored and the independent styles still evaluate.
    assert_eq!(output.dependencies, Vec::<String>::new());
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut sources = Sources(Vec::new());
    sources.visit_program(&parsed.program);
    let units = if literal == r"'\uD800'" {
        vec![0xD800]
    } else {
        vec![0xDC00]
    };
    assert_eq!(sources.0, vec![units.clone(), units]);
    let styles: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    assert_eq!(styles, vec![("padding", "16px")]);
    assert!(!output.code.contains("css("), "{}", output.code);
}

#[test]
fn surrogate_destructured_prop_reads_require_whole_props() {
    // Given a prop name that cannot be represented by the UTF-8 forwarding list.
    let allocator = Allocator::default();
    let expression = Parser::new(
        &allocator,
        r"({'\uD800': value, color}) => value",
        SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    // When the styled forwarding analysis reads the callback.
    let mut reads = crate::styled_reads::Reads::default();
    reads.read_in(&expression);
    // Then it retains whole-props semantics rather than conflating surrogate keys.
    assert_eq!(
        reads,
        crate::styled_reads::Reads {
            names: vec!["color".to_string()],
            whole: true
        }
    );
}

#[test]
fn surrogate_css_text_defers_declaration_scanning_but_layer_validation_rejects_it() {
    // Given unrepresentable text containing an otherwise forbidden declaration.
    let allocator = Allocator::default();
    let expression = Parser::new(
        &allocator,
        r"'box-orient: vertical; content: \uD800'",
        SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    // When optional declaration scanning and mandatory text validation consume it.
    let declarations = crate::dead_properties::expression_errors(&expression);
    let errors = crate::css_utils::expression_layer_errors(&expression, "css");
    // Then no invented declaration offsets escape, but the literal has a fatal error.
    assert_eq!(declarations.len(), 0);
    assert_eq!(errors, vec![(0, "`css()` cannot use `\"box-orient: vertical; content: \\ud800\"` at build time: CSS text must contain no unpaired UTF-16 surrogate".to_string())]);
}

#[rstest]
#[case(r"'box-orient: vertical; content: \uD800'")]
#[case(r"'box-orient: vertical; content: \uDC00'")]
#[serial]
fn unrepresentable_root_css_fails_at_the_literal_before_declaration_scanning(
    #[case] literal: &str,
) {
    // Given a root CSS string whose text cannot be consumed as UTF-8.
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css }} from '@devup-ui/react';\ncss({literal});");
    // When the extraction guard runs before the root-string and declaration readers.
    let error = extract("boundary.tsx", &source, ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("expected invalid CSS text"))
        .to_string();
    // Then the authored literal owns the fatal surrogate error, not a guessed CSS key.
    assert!(error.starts_with("boundary.tsx:2:5:"), "{error}");
    assert!(
        error.contains("CSS text and values must contain no unpaired UTF-16 surrogate"),
        "{error}"
    );
    assert!(!error.contains("is obsolete"), "{error}");
}

#[rstest]
#[case(r"export { Box as Other } from '\uD800';")]
#[case(r"export * from '\uDC00';")]
#[serial]
fn barrel_resolution_ignores_unrepresentable_reexport_sources(#[case] reexport: &str) {
    // Given a barrel with both an unrelated opaque export and a valid Box export.
    reset_class_map();
    reset_file_map();
    let code = format!("{reexport}\nexport {{ Box }} from '@devup-ui/react';");
    let resolver = move |source: &str, _: &str| {
        assert_ne!(source, "\u{FFFD}");
        (source == "./ui").then(|| crate::ResolvedModule {
            path: "/ui.ts".to_string(),
            code: code.clone(),
        })
    };
    // When the barrel is followed for its valid export.
    let output = extract_with_modules(
        "/app.tsx",
        "import { Box } from './ui'; export const result = <Box p={2} />;",
        ExtractOption::default(),
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then unrelated surrogate sources neither poison nor replace the valid export.
    assert_eq!(output.dependencies, vec!["/ui.ts"]);
    assert!(output.code.contains("<div className="), "{}", output.code);
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px")));
}

#[test]
#[serial]
fn namespace_destructuring_with_surrogate_key_reports_the_whole_read() {
    // Given a noncomputed static key that cannot name a configured UTF-8 export.
    reset_class_map();
    reset_file_map();
    let source = "import * as UI from '@devup-ui/react';\nconst { '\\uD800': value } = UI;\nexport { value };";
    // When the namespace rewriter sees the destructuring.
    let error = extract("namespace.tsx", source, ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("expected namespace error"))
        .to_string();
    // Then it fails at the authored whole-read boundary, never a replacement export.
    assert!(error.starts_with("namespace.tsx:2:7:"), "{error}");
    assert!(error.contains("`UI` cannot use"), "{error}");
    assert!(error.contains(r"\uD800"), "{error}");
}
