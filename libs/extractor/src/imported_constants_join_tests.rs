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

#[test]
#[serial_test::serial]
fn scoped_captures_when_exact_dependencies_and_raw_copies_join_keep_their_owners()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: one module mixes captured rules, exact primitives and copy-only versus escaped objects.
    let source = r"import {Box,styled} from '@devup-ui/react';
enum E { A=2, B=A+1 }
const trace=[];
function make(input){const rest=input.color;const __devupValue0='collision';const first=Math.imul(E.B,2);const gap=first+1;const zero=Math.sign(-0);const copy={p:gap};const changed={color:'raw'};const alias=changed;alias.color='mutated';const local={color:rest,_hover:{opacity:input.opacity}};const callback=p=>({...local,m:gap});const Card=styled(Box)(callback);const view=<Box zIndex={E.B} {...copy} {...changed} title={__devupValue0} data-zero={zero} data-object={changed}/>;return {Card,view,changed,alias};}
function shadowed(Math,undefined){const value=Math.imul(undefined,2);return <Box zIndex={value}/>;}
";
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let output = crate::extract(
        "joined.tsx",
        source,
        ExtractOption {
            single_css: true,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )?;
    // When: both renderers read the original captured object after the initializer has run once.
    let actual = crate::styled_source_tests::captured_render::execute(
        ("joined.tsx", None),
        &output,
        "const made=make({get color(){trace.push('color');return 'navy';},get opacity(){trace.push('opacity');return 0.5;}});const first=render({target:made.Card,props:{}});const second=render({target:made.Card,props:{}});const local=shadowed({imul(a,b){trace.push('math');return a+b;}},9);JSON.stringify([[first,second].map(e=>[values(e,'color'),values(e,'opacity',0,'&:hover'),values(e,'margin')]),values(made.view,'padding'),values(made.view,'z-index'),made.view.props.title,Object.is(made.view.props['data-zero'],-0),made.view.props['data-object']===made.changed,made.changed===made.alias,made.view.props.color,values(local,'z-index'),trace]);",
    );
    // Then: shadowed globals and raw object identity survive while only proven styles are extracted.
    assert_eq!(
        actual,
        serde_json::json!([
            [[["navy"], [0.5], ["28px"]], [["navy"], [0.5], ["28px"]]],
            ["28px"],
            ["3"],
            "collision",
            true,
            true,
            true,
            "mutated",
            [11],
            ["color", "opacity", "math"]
        ])
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn initialization_failures_when_namespace_aliases_rewrite_keep_authored_locations()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: namespace and import-alias edits precede an immediate lexical read.
    let source = concat!(
        "import * as emotion from '@emotion/css';\n",
        "export const rule=emotion.css({color:late});\nconst late='green';"
    );
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/css".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When: the original read is terminal, rather than a candidate for value evaluation.
    let error = crate::extract("joined-tdz.tsx", source, option.clone())
        .err()
        .ok_or("TDZ compiled")?
        .to_string();
    let offset = source.find("late").ok_or("TDZ use")?;
    // Then: the error names the authored use; moving the read into a deferred body succeeds.
    assert!(
        error.starts_with(&format!(
            "{}:",
            crate::locate("joined-tdz.tsx", source, offset)
        )),
        "{error}"
    );
    assert!(error.contains("ReferenceError"), "{error}");
    let deferred = concat!(
        "import * as emotion from '@emotion/css';\n",
        "export const rule=()=>emotion.css({color:late});\nconst late='green';"
    );
    let output = crate::extract("joined-tdz.tsx", deferred, option.clone())?;
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property()=="color" && style.value()=="green")));
    // Given: an actual evaluator edit precedes the same namespace/alias TDZ suffix.
    let prefix = "import {css} from '@devup-ui/react';function double(n){return n*2;}const earlier=css({width:double(12345)});\n";
    let (computed, edits, _) = crate::build_time_values::evaluate_located(
        prefix,
        "joined-tdz.tsx",
        &option,
        None,
        &Unknown::default(),
    )
    .map_err(|errors| format!("{errors:?}"))?
    .ok_or("missing evaluated prefix")?;
    assert_ne!(edits, [] as [(usize, usize, usize); 0]);
    assert_ne!(prefix.len(), computed.len());
    let original = format!("{prefix}{source}");
    let evaluated = format!("{computed}{source}");
    // When: the recursive extraction seam receives the real earlier replacement map.
    let error = crate::extract_source(
        "joined-tdz.tsx",
        &evaluated,
        Some((&original, &[edits.as_slice()])),
        None,
        option,
        false,
        None,
    )
    .err()
    .ok_or("TDZ compiled")?
    .to_string();
    // Then: the terminal ReferenceError names the original authored late read.
    let offset = original.find("late").ok_or("TDZ use")?;
    assert!(
        error.starts_with(&format!(
            "{}:",
            crate::locate("joined-tdz.tsx", &original, offset)
        )),
        "{error}"
    );
    assert!(error.contains("ReferenceError"), "{error}");
    Ok(())
}

#[test]
#[serial_test::serial]
fn recoverable_local_errors_when_a_later_value_is_forbidden_reach_the_located_sandbox()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: local ownership needs fallback, and a separate computed value reads a forbidden helper.
    let source = "import * as emotion from '@emotion/css';\nconst compare=Object.getOwnPropertyDescriptor(String.prototype,'localeCompare').value;\nexport function f(props){const inner={color:props.color};observe(inner);return emotion.css(inner);}\nexport const later=emotion.css({width:compare('a','b')});";
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/css".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    // When: public extraction passes recoverable inliner errors to the ordinary fallback pipeline.
    let error = crate::extract("joined-fallback.tsx", source, option)
        .err()
        .ok_or("forbidden helper compiled")?
        .to_string();
    // Then: a blanket early return cannot hide the later helper or remap it to generated source.
    let offset = source
        .find("Object.getOwnPropertyDescriptor")
        .ok_or("helper fixture")?;
    assert!(
        error.lines().any(|line| line.starts_with(&format!(
            "{}:",
            crate::locate("joined-fallback.tsx", source, offset)
        ))),
        "{error}"
    );
    assert!(error.contains("String.prototype.localeCompare"), "{error}");
    Ok(())
}
