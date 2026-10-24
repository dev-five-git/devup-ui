use super::*;
use crate::{
    extract_style::extract_style_value::ExtractStyleValue, gen_class_name::gen_class_names,
};

#[rstest::rstest]
#[case::keep(false, "12px")]
#[case::replace(true, "16px")]
#[serial_test::serial]
fn sequential_chunks_keep_previous_atom_when_later_branch_omits_key(
    #[case] on: bool,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the expression extractor admits a conditional scalar before any capture lowering.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = oxc_parser::Parser::new(
        &allocator,
        "({padding:3,p:on?4:null})",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .map_err(|error| format!("{error:?}"))?;
    // When: the same traversal produces legacy emission and sequential authored chunks.
    let (_, authored) = extract_with_order(
        &ast,
        None,
        &mut expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    );
    let mut normalized = authored.normalized(&ast).0;
    let class = gen_class_names(&ast, &mut normalized, None, None).ok_or("missing class")?;
    let code = format!("const on={on};{}", crate::utils::expression_to_code(&class));
    let selected = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&code))
        .map_err(|error| error.to_string())?
        .as_string()
        .ok_or("class must be a string")?
        .to_std_string_escaped();
    // Then: Keep resolves against the earlier alias rather than an empty composition.
    let values: Vec<_> = normalized
        .iter()
        .flat_map(ExtractStyleProp::extract)
        .filter_map(|value| {
            let class = value.extract(None)?;
            match (value, class) {
                (
                    ExtractStyleValue::Static(style),
                    crate::extract_style::style_property::StyleProperty::ClassName(name),
                ) if selected.split_whitespace().any(|selected| selected == name) => {
                    Some(style.value().to_string())
                }
                _ => None,
            }
        })
        .collect();
    assert_eq!(values, vec![expected]);
    Ok(())
}

#[rstest::rstest]
#[case::keep(false, "12px", "20px")]
#[case::replace(true, "16px", "24px")]
#[serial_test::serial]
fn located_dynamic_leaf_keeps_authored_conditional_layer_winners(
    #[case] on: bool,
    #[case] plain: &str,
    #[case] layered: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: aliases compete independently inside and outside a layer.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut expression = oxc_parser::Parser::new(
        &allocator,
        "({padding:3,p:on?4:null,'@layer':{base:{padding:5,p:on?6:null}},color:tone})",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .map_err(|error| format!("{error:?}"))?;
    let mut prepared = 0;
    // When: the combined entry locates leaves while retaining authored chunks.
    let (result, authored) = extract_with_order_policy::<true>(
        &ast,
        None,
        &mut expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
        &mut |_| prepared += 1,
    );
    // Then: both views retain the exact leaf location, not its containing object.
    assert_eq!(prepared, 1);
    for styles in [&result.styles, &authored.0] {
        let mut unreadable = Vec::new();
        crate::utils::unreadable_styles(styles, false, &mut unreadable);
        assert_eq!(unreadable, vec![(70, "tone".to_string(), None)]);
    }
    let mut normalized = authored.normalized(&ast).0;
    let class = gen_class_names(&ast, &mut normalized, None, None).ok_or("missing class")?;
    let code = format!("const on={on};{}", crate::utils::expression_to_code(&class));
    let selected = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&code))
        .map_err(|error| error.to_string())?
        .as_string()
        .ok_or("class must be a string")?
        .to_std_string_escaped();
    let mut winners: Vec<_> = normalized
        .iter()
        .flat_map(ExtractStyleProp::extract)
        .filter_map(|value| {
            let class = value.extract(None)?;
            match (value, class) {
                (
                    ExtractStyleValue::Static(style),
                    crate::extract_style::style_property::StyleProperty::ClassName(name),
                ) if selected.split_whitespace().any(|selected| selected == name) => {
                    Some((style.layer, style.property, style.value))
                }
                _ => None,
            }
        })
        .collect();
    winners.sort();
    assert_eq!(
        winners,
        vec![
            (None, "padding".to_string(), plain.to_string()),
            (
                Some("base".to_string()),
                "padding".to_string(),
                layered.to_string(),
            ),
        ]
    );
    Ok(())
}

#[rstest::rstest]
#[case::object("({value:motion})['value']")]
#[case::array("[motion][0]")]
#[serial_test::serial]
fn preparation_reaches_nested_shorthand_member_siblings_with_original_binding(
    #[case] member: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: semantic references survive the clones used for shorthand siblings.
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = format!("const motion = null; ({{_hover:{{mx:{member}}}}});");
    let mut parsed =
        oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = std::rc::Rc::new(
        oxc_semantic::SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping(),
    );
    let mut values = crate::style_values::StyleValues::new(std::rc::Rc::clone(&scoping));
    let symbol = values
        .root_symbol("motion")
        .ok_or("missing motion binding")?;
    let reference = *scoping
        .get_resolved_reference_ids(symbol)
        .first()
        .ok_or("missing motion reference")?;
    values.insert(
        symbol,
        crate::style_values::StyleValue::Keyframes("pulse".to_string()),
    );
    let oxc_ast::ast::Statement::ExpressionStatement(statement) =
        parsed.program.body.last_mut().ok_or("missing rules")?
    else {
        return Err("expected rules expression".into());
    };
    let mut prepared = Vec::new();
    // When: real binding normalization runs at the combined seam's prepare hook.
    let (_, authored) = extract_with_order_policy::<false>(
        &ast,
        None,
        &mut statement.expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
        &mut |value| {
            let reference = match value {
                Expression::Identifier(identifier) => identifier.reference_id.get(),
                _ => None,
            };
            prepared.push((values.symbol(value), reference));
            values.read_in(&ast, value);
        },
    );
    // Then: each sibling sees the original binding before its value is serialized.
    assert_eq!(prepared, vec![(Some(symbol), Some(reference)); 2]);
    let mut observed = Vec::new();
    for value in authored
        .normalized(&ast)
        .0
        .iter()
        .flat_map(ExtractStyleProp::extract)
    {
        let ExtractStyleValue::Dynamic(style) = value else {
            return Err("expected prepared dynamic sibling".into());
        };
        let value = boa_engine::Context::default()
            .eval(boa_engine::Source::from_bytes(style.identifier()))
            .map_err(|error| error.to_string())?
            .as_string()
            .ok_or("prepared value must be a string")?
            .to_std_string_escaped();
        observed.push((
            style.property().to_string(),
            style.selector().map(ToString::to_string),
            value,
        ));
    }
    observed.sort();
    assert_eq!(
        observed,
        vec![
            (
                "margin-left".to_string(),
                Some("&:hover".to_string()),
                "pulse".to_string(),
            ),
            (
                "margin-right".to_string(),
                Some("&:hover".to_string()),
                "pulse".to_string(),
            ),
        ]
    );
    Ok(())
}
