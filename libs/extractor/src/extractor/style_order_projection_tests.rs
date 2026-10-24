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
