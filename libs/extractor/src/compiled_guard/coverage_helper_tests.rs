use std::error::Error;

use oxc_allocator::Allocator;
use oxc_ast::ast::{PropertyDefinition, Statement};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serde_json::{Value, json};

use super::{
    compiled_guard_facts_internal, diagnostics,
    origins::{export_name, local_name, property_name},
    scopes::Scopes,
    text::Text,
    traversal::Collector,
    wire::{Facts, Origin, Positions},
};

#[rstest]
#[case("a\r\n😀x", (2, 4))]
#[case("a\rb\nc\u{2028}d\u{2029}😀", (5, 3))]
#[case("a\r\n\n", (3, 1))]
fn location_when_line_endings_and_astral_characters_are_mixed(
    #[case] code: &str,
    #[case] expected: (usize, usize),
) -> Result<(), Box<dyn Error>> {
    // Given: byte positions at the end of text with JS line terminators.
    let offset = u32::try_from(code.len())?;
    // When: diagnostics convert bytes to line and UTF-16 column.
    let result = diagnostics::location(code, offset);
    // Then: CRLF counts once and astral characters count two columns.
    assert_eq!(result, expected);
    Ok(())
}

#[rstest]
#[case("\r\n")]
#[case("\r")]
#[case("\n")]
#[case("\u{2028}")]
#[case("\u{2029}")]
fn diagnostic_when_invalid_js_follows_a_line_terminator(#[case] ending: &str) {
    // Given: valid astral text followed by a syntactically invalid declaration.
    let code = format!("let emoji='😀';{ending}const value = ;");
    // When: the public facts API rejects the actual parser error.
    let result = compiled_guard_facts_internal(&code, false);
    // Then: the diagnostic locates the bad token on the second line.
    assert!(matches!(result, Err(error) if error.to_string().starts_with("compiled output:2:15:")));
}

#[test]
fn property_names_when_parser_produces_private_and_computed_keys() {
    #[derive(Default)]
    struct Keys(Vec<Option<Text>>);
    impl<'a> Visit<'a> for Keys {
        fn visit_property_definition(&mut self, node: &PropertyDefinition<'a>) {
            self.0.push(property_name(&node.key, node.computed));
        }
    }
    // Given: real class field keys, including a dynamic computed key.
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        "class C{#secret; plain; ['literal']; [dynamic];}",
        SourceType::jsx(),
    )
    .parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    let mut keys = Keys::default();
    // When: the naming helper reads actual key kinds and computed flags.
    keys.visit_program(&parsed.program);
    // Then: private names retain their name and dynamic keys remain unknown.
    assert_eq!(
        keys.0,
        vec![
            Some(Text::from("secret")),
            Some(Text::from("plain")),
            Some(Text::from("literal")),
            None
        ]
    );
}

#[test]
fn module_names_when_local_and_exported_ast_variants_differ() -> Result<(), Box<dyn Error>> {
    // Given: a local export with an IdentifierReference and IdentifierName.
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        "const css=0; export {css as renamed};",
        SourceType::jsx(),
    )
    .parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    let Statement::ExportNamedDeclaration(node) = &parsed.program.body[1] else {
        return Err("named export node".into());
    };
    let specifier = &node.specifiers[0];
    // When: the helpers read both parser-provided name variants.
    let result = (
        export_name(&specifier.local),
        local_name(&specifier.exported),
    );
    // Then: names agree without treating the exported alias as the local binding.
    assert_eq!(result, (Text::from("css"), Some("renamed")));
    Ok(())
}

#[test]
fn local_name_when_forwarded_export_uses_a_string_literal() -> Result<(), Box<dyn Error>> {
    // Given: a valid string-named forwarding export, not a local binding.
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        "export {'css' as renamed} from 'ui';",
        SourceType::jsx(),
    )
    .parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    let Statement::ExportFromDeclaration(node) = &parsed.program.body[0] else {
        return Err("forwarded export node".into());
    };
    // When: the local-name helper sees the real string variant.
    let result = local_name(&node.specifiers[0].local);
    // Then: a string export name cannot masquerade as a lexical binding.
    assert_eq!(result, None);
    Ok(())
}

#[rstest]
#[case("export interface Shape{}", json!([]))]
#[case("export class Shape{}", json!([["Shape", null]]))]
fn declared_exports_when_parser_declarations_are_type_only_or_classes(
    #[case] code: &str,
    #[case] expected: Value,
) -> Result<(), Box<dyn Error>> {
    // Given: parser-created declarations for the helper's Declaration input type.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    let Statement::ExportDeclaration(node) = &parsed.program.body[0] else {
        return Err("export declaration node".into());
    };
    let mut collector = Collector::new(Scopes::build(&parsed.program), Positions::new(code), false);
    // When: the internal declaration helper collects runtime export names.
    collector.declared_export(&node.declaration);
    // Then: class declarations are named and type-only declarations add nothing.
    let result: Value = serde_json::from_str(&collector.facts.json())?;
    assert_eq!(result["exports"], expected);
    Ok(())
}

#[test]
fn wire_export_when_existing_name_is_replaced_after_another_insertion() -> Result<(), Box<dyn Error>>
{
    // Given: the ordered wire accumulator with two distinct exported names.
    let mut facts = Facts::new(false);
    facts.export(Text::from("first"), None);
    facts.export(Text::from("second"), None);
    let origin = Origin {
        request: Text::from("ui"),
        ids: vec![Text::from("css")],
        require: true,
    };
    // When: a later fact supplies provenance for an already inserted name.
    facts.export(Text::from("first"), Some(origin));
    // Then: the origin changes without duplication or moving its original slot.
    let result: Value = serde_json::from_str(&facts.json())?;
    assert_eq!(
        result["exports"],
        json!([
            ["first", {"request":"ui","ids":["css"],"kind":"require-call"}], ["second", null]
        ])
    );
    Ok(())
}
