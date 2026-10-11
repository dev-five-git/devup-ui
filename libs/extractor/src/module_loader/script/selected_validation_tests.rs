use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use rstest::rstest;

use super::super::Unit;
use crate::ImportAlias;
use crate::import_alias_visit::transform_import_aliases_with_edits;
use crate::module_loader::Mapped;
use crate::vanilla_extract::Stylesheet;

#[rstest]
#[case("const broken = ;\n", "/selected.css.ts:3:16")]
#[case("let duplicate=1;\nlet duplicate=2;\n", "/selected.css.ts:3:5")]
fn selected_validation_keeps_original_labels_when_aliases_and_copied_ranges_move_code(
    #[case] suffix: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given
    let filename = "/selected.css.ts";
    let prefix = "import {style} from '@vanilla-extract/css';\nconst label: string='한글😀';\n";
    let original = format!("{prefix}{suffix}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &original, SourceType::ts()).parse();
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .build(&parsed.program);
    let diagnostic = parsed
        .diagnostics
        .first()
        .or_else(|| built.diagnostics.first())
        .ok_or("invalid authored fixture has no diagnostic")?;
    let label = diagnostic
        .labels
        .first()
        .ok_or("authored diagnostic has no label")?;
    let offset = usize::try_from(label.offset()).map_err(|error| error.to_string())?;
    let place = crate::locate(filename, &original, offset);
    assert_eq!(place, expected);
    let aliases = std::collections::HashMap::from([(
        "@vanilla-extract/css".into(),
        ImportAlias::NamedToNamed,
    )]);
    let aliased =
        transform_import_aliases_with_edits(prefix, filename, "@devup-ui/react", &aliases);
    let code = format!("{}{suffix}", aliased.code);
    assert_ne!(code, original);
    assert_ne!(aliased.edits.len(), 0);
    let start = code
        .find("const label")
        .ok_or("missing copied authored declaration")?;
    let span = Span::new(
        u32::try_from(start).map_err(|error| error.to_string())?,
        u32::try_from(code.len()).map_err(|error| error.to_string())?,
    );
    let mut mapped = Mapped::default();
    mapped.synthesize(0, "/* selection */\n");
    mapped.copy(&code, span);
    let edits = [aliased.edits.as_slice()];
    let stylesheet = Stylesheet {
        filename,
        code: &code,
        source: &original,
        edits: &edits,
    };
    // When
    let error = Unit::selected(stylesheet, &mapped)
        .err()
        .ok_or("invalid selected source was accepted")?;
    // Then
    let prefix = format!("{expected}: JS execution error: SyntaxError:");
    assert!(error.starts_with(&prefix), "{error}");
    let fix = "Fix: correct the syntax at this location";
    assert!(error.contains(fix), "{error}");
    Ok(())
}
