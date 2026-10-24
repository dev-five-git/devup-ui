use crate::{
    composition::Composition,
    extractor::{
        ExtractResult,
        extract_style_from_expression::LiteralHandling,
        extract_style_from_styled::StyledBindings,
        style_order_projection::{AuthoredStyles, extract_with_order},
    },
};
use oxc_ast::{ast::Expression, builder::AstBuilder};

pub(super) fn extract<'a>(
    ast: &AstBuilder<'a>,
    rules: &mut Expression<'a>,
    (bindings, filename): (Option<&StyledBindings<'_>>, &str),
) -> ExtractResult<'a> {
    if let Some(styles) = bindings.and_then(|bindings| bindings.styles(rules)) {
        return ExtractResult {
            styles: AuthoredStyles(
                styles
                    .iter()
                    .cloned()
                    .map(crate::ExtractStyleProp::Static)
                    .collect(),
            )
            .registered(filename)
            .0,
            ..ExtractResult::default()
        };
    }
    if let Expression::ConditionalExpression(branch) = crate::utils::unwrap_syntax_only_mut(rules) {
        let yes = extract(ast, &mut branch.consequent, (bindings, filename));
        let no = extract(ast, &mut branch.alternate, (bindings, filename));
        let mut composition = Composition::default();
        composition.apply_conditional(
            ast,
            &branch.test,
            AuthoredStyles(yes.styles).normalized(ast).0,
            AuthoredStyles(no.styles).normalized(ast).0,
        );
        return ExtractResult {
            styles: composition.into_props(),
            ..ExtractResult::default()
        };
    }
    let (mut result, authored) = extract_with_order(
        ast,
        None,
        rules,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    );
    result.styles = authored.registered(filename).0;
    result
}
