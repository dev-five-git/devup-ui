use super::*;
use rstest::rstest;

#[test]
fn compiled_css_shorthand_when_inlined_keeps_the_css_property_name() {
    // Given: a real local rule constant is passed as a compiled JSX shorthand property.
    let arena = Allocator::default();
    let source =
        "import {jsx} from '@emotion/react';const css={ margin: 2 };const view=jsx('div',{css});";
    let mut parsed = Parser::new(&arena, source, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let options = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..Default::default()
    };
    // When: ordinary constant inlining rewrites the value at the compiled CSS-prop root.
    let result = inline_constants(
        &AstBuilder::new(&arena),
        &mut parsed.program,
        "shorthand.ts",
        &options,
        None,
        CssProp::Everywhere,
        &FxHashSet::default(),
    );
    assert_eq!(result.errors, vec![]);
    parsed
        .program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!(
        "function jsx(target,props){{return {{target,props}};}}{code};JSON.stringify(view);"
    );
    let actual = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON observation"))
        .to_std_string_escaped();
    // Then: the rewritten value is valid JavaScript under the original css key, with Emotion pixels.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("{error}")),
        serde_json::json!({"target":"div","props":{"css":{"margin":"2px"}}})
    );
}

#[test]
fn pixel_rules_when_nested_selectors_are_arrays_preserve_unitless_values()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: selector arrays retain numbers for native extraction to scale later.
    let allocator = Allocator::default();
    let mut parsed = Parser::new(
        &allocator,
        "const rules={_hover:[{margin:2,lineHeight:3}],vars:{'--raw':4}};",
        SourceType::tsx(),
    )
    .parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Some(Statement::VariableDeclaration(declaration)) = parsed.program.body.first_mut() else {
        return Err("missing rules declaration".into());
    };
    let rules = declaration.declarations[0]
        .init
        .as_mut()
        .ok_or("missing rules initializer")?;
    // When: the production pixel traversal prepares the complete rule tree.
    px_rules(&AstBuilder::new(&allocator), rules);
    let generated = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!("{generated}\nJSON.stringify(rules);");
    let json = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| error.to_string())?
        .as_string()
        .ok_or("expected JSON result")?
        .to_std_string_escaped();
    // Then: preparation leaves selector-array values and vars untouched.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json)?,
        serde_json::json!({"_hover":[{"margin":2,"lineHeight":3}],"vars":{"--raw":4}})
    );
    Ok(())
}

#[rstest]
#[case("inner")]
#[case("rest")]
fn shape_accounting_when_selection_erases_a_read_counts_original_sites(
    #[case] name: &str,
    #[values(false, true)] observed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: expansion manufactures several member reads from one original site.
    let allocator = Allocator::default();
    let observation = if observed {
        format!("observe({name});")
    } else {
        String::new()
    };
    let source = format!(
        "import {{styled}} from '@devup-ui/react'; function f(props) {{ const {name} = {{color:props.color,_hover:{{color:props.hover}}}}; {observation} return styled.div(true ? {name} : {name}); }}"
    );
    let mut parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let offset = u32::try_from(source.find(&format!("{name} =")).ok_or("missing binding")?)?;
    // When: selection erases one site and shape expansion consumes the other.
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "join.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    // Then: generated members neither hide an extra use nor invent one.
    let errors = if observed {
        vec![(offset, crate::utils::local_style_error(name))]
    } else {
        vec![]
    };
    assert_eq!(inlined.errors, errors);
    Ok(())
}

#[test]
fn scoping_when_a_bound_callback_captures_a_local_is_returned_for_reuse()
-> Result<(), Box<dyn std::error::Error>> {
    struct Reads<'s> {
        scoping: &'s Scoping,
        symbols: Vec<SymbolId>,
    }
    impl<'a> Visit<'a> for Reads<'_> {
        fn visit_identifier_reference(
            &mut self,
            identifier: &oxc_ast::ast::IdentifierReference<'a>,
        ) {
            if identifier.name == "__devup_rest"
                && let Some(symbol) = binding_of(self.scoping, identifier)
            {
                self.symbols.push(symbol);
            }
        }
    }
    // Given: the callback and a generated capture alias must share original IDs.
    let allocator = Allocator::default();
    let source = "import {styled} from '@devup-ui/react'; function f(props) { const rest=props.color; const rules=p=>({ color: rest }); return styled.div(rules); }";
    let mut parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When: the callback is cloned and its captured binding is renamed.
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut parsed.program,
        "join.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &FxHashSet::default(),
    );
    // Then: the caller receives the semantic phase that owns the renamed reads.
    assert_eq!(inlined.errors, vec![]);
    let scoping = inlined.scoping.ok_or("missing shared scoping")?;
    let symbol = scoping
        .symbol_ids()
        .find(|symbol| scoping.symbol_name(*symbol) == "rest")
        .ok_or("missing captured symbol")?;
    let mut reads = Reads {
        scoping: &scoping,
        symbols: Vec::new(),
    };
    reads.visit_program(&parsed.program);
    assert_eq!(reads.symbols, vec![symbol, symbol]);
    Ok(())
}
