use crate::ExtractStyleProp;
use crate::extractor::extract_style_from_expression::yield_typography;
use oxc_ast::ast::Expression;
use oxc_ast::builder::AstBuilder;

pub(crate) mod emission;
pub(crate) mod roots;

pub fn gen_class_names<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    yield_typography(style_props);
    merge_expression_for_class_name(
        ast_builder,
        style_props
            .iter_mut()
            .filter_map(|st| gen_class_name(ast_builder, st, style_order, filename))
            .rev(),
    )
}

fn gen_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &mut ExtractStyleProp<'a>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    emission::emit_prop(
        ast_builder,
        style_prop,
        emission::ClassPlacement {
            order: style_order,
            filename,
        },
    )
    .map(roots::products::Generated::into_expression)
}

pub fn merge_expression_for_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: impl IntoIterator<Item = Expression<'a>>,
) -> Option<Expression<'a>> {
    emission::merge_roots(ast_builder, expressions)
}
