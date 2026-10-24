use super::*;

#[rstest]
#[case::discarded_alternate("css(true ? holder.rules : holder.other)")]
#[case::discarded_consequent("css(false ? holder.other : holder.rules)")]
#[case::and_right("css(holder.on && holder.rules)")]
#[case::and_left("css(holder.off && holder.other, holder.rules)")]
#[case::or_boolean_left("css(holder.on || holder.other, holder.rules)")]
#[case::or_left("css(holder.rules || holder.other)")]
#[case::or_right("css(holder.off || holder.rules)")]
#[case::nullish_left("css(holder.rules ?? holder.other)")]
#[case::nullish_right("css(holder.none ?? holder.rules)")]
#[case::nested_choices("css(holder.on ? (holder.none ?? holder.rules) : holder.other)")]
#[case::discarded_nested("css(holder.rules, false ? holder.other : {color:'red'})")]
#[case::repeated_computed("css(holder[holder.key], holder[holder.key])")]
#[case::repeated_operands("css({color:holder.base+holder.suffix+holder.empty})")]
#[case::template_operands("css({color:`${holder.base}${holder.suffix}${holder.empty}`})")]
#[serial]
fn style_only_reads_compile_when_selection_erases_other_references(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: all semantic references belong to the style expression, including discarded sides.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nexport function f(){{const holder={{on:true,off:false,none:null,key:'rules',base:'r',suffix:'ed',empty:'',rules:{{color:'red'}},other:{{color:'blue'}}}};return {expression};}}"
    );
    // When: the shared inliner folds the expression before style extraction.
    let output = extract(
        "local-accounting.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{expression}: {error}"));
    // Then: only the selected red declaration is emitted.
    let colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => Some(style.value()),
            _ => None,
        })
        .collect();
    assert_eq!(colors, vec!["red"], "{expression}");
}

#[rstest]
#[case::discarded_only("css(true ? {color:'red'} : holder.rules)")]
#[case::discarded_consequent("css(false ? holder.rules : {color:'red'})")]
#[case::or_retained_left("css(holder.rules || holder.other)")]
#[case::nullish_retained_left("css(holder.rules ?? holder.other)")]
#[case::nullish_right("css(holder.none ?? holder.rules)")]
#[case::and_discarded_right("css({color:holder.color}, false && holder.rules)")]
#[case::repeated_computed("css(holder[holder.key], holder[holder.key])")]
#[case::folded_math("css({width:Math.max(holder.size,holder.size)})")]
#[case::folded_template("css({color:`${holder.color}${holder.empty}`})")]
#[serial]
fn local_guard_keeps_its_location_when_erased_reads_have_an_extra_use(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: observation is a distinct semantic reference outside the style expression.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nexport function f(){{const holder={{none:null,key:'rules',color:'red',empty:'',size:2,rules:{{color:'red'}},other:{{color:'blue'}}}};observe(holder);return {expression};}}"
    );
    // When: successful folding or selection consumes the style references.
    let result = extract(
        "local-accounting.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    let error = match result {
        Ok(output) => panic!("erased local references must retain the extra-use error: {output:?}"),
        Err(error) => error.to_string(),
    };
    // Then: the original binding location and refusal remain authoritative.
    assert!(error.starts_with("local-accounting.ts:2:27: "), "{error}");
    assert!(
        error.contains(&crate::utils::local_style_error("holder")),
        "{error}"
    );
}

#[rstest]
#[case::sibling("export function other(){const holder={rules:{color:'blue'}};observe(holder);}")]
#[case::shadow("export function other(holder){observe(holder);}")]
#[case::outer("const holder={rules:{color:'blue'}};observe(holder);")]
#[serial]
fn accounting_uses_binding_identity_when_names_are_shared(
    #[case] adjacent: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: an unrelated binding shares the spelling of the style-only local.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css}} from '@devup-ui/react';{adjacent}\nexport function f(){{const holder={{key:'rules',rules:{{color:'red'}}}};return css(holder[holder.key]);}}"
    );
    // When: references are resolved across sibling and nested scopes.
    let output = extract(
        "local-accounting.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then: the unrelated observation neither refuses nor changes the style-only binding.
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "red"
    )));
}

#[rstest]
#[case::calls("return [nativeCss(holder.rules),css(holder.rules)];")]
#[case::elements("return <><Box {...holder.rules}/><div css={holder.rules}/></>;")]
#[case::class_names(
    "return <><Box {...holder.rules}/><ClassNames>{({css})=><div className={css(holder.rules)}/>}</ClassNames></>;"
)]
#[serial]
fn adjacent_consumers_keep_units_when_they_share_a_local_member(
    #[case] body: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: native and library consumers read the same local object with different units.
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css as nativeCss,Box}} from '@devup-ui/react';import {{css,ClassNames}} from '@emotion/react';\nexport function f(){{const holder={{rules:{{color:'red',margin:2}}}};{body}}}"
    );
    // When: folded members pass through their original numeric-context handling.
    let output = extract(
        "local-accounting.tsx",
        &source,
        ExtractOption {
            single_css,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let mut margins: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "margin" => Some(style.value()),
            _ => None,
        })
        .collect();
    margins.sort_unstable();
    // Then: library pixels and native scaled pixels both survive.
    assert_eq!(margins, vec!["2px", "8px"]);
}

#[rstest]
#[case::computed("css(holder[runtime])")]
#[case::selected_runtime("css(true ? holder[runtime] : {color:'red'})")]
#[case::failed_math("css({width:Math.max('unknown',holder)})")]
fn runtime_references_stay_uncounted_when_folding_cannot_erase_them(#[case] expression: &str) {
    // Given: a local object has runtime uses but no successfully folded style read.
    let allocator = oxc_allocator::Allocator::default();
    let source = format!(
        "import {{css}} from '@devup-ui/react';function f(runtime){{const holder={{rules:{{color:'red'}}}};observe(holder);return {expression};}}"
    );
    let mut program = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::ts())
        .parse()
        .program;
    // When: only the shared constant-inlining pass runs, without downstream evaluation.
    let inlined = crate::imported_constants::inline_constants(
        &oxc_ast::builder::AstBuilder::new(&allocator),
        &mut program,
        "local-accounting.ts",
        &ExtractOption::default(),
        None,
        crate::css_prop::CssProp::Off,
        &Default::default(),
    );
    // Then: probes and still-runtime selected subtrees do not activate the extra-use policy.
    assert_eq!(inlined.errors, vec![]);
}
