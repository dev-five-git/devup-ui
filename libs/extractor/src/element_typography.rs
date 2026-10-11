use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, Str},
    builder::AstBuilder,
};
use oxc_span::{GetSpan, GetSpanMut};

use crate::{ExtractStyleProp, utils::get_string_by_literal_expression};

#[path = "typography_scalar_projection.rs"]
mod scalar_projection;

pub(crate) fn capture<'a>(
    ast: &AstBuilder<'a>,
    styles: &mut Vec<ExtractStyleProp<'a>>,
    source: &Expression<'a>,
    next: &mut usize,
) -> Option<(String, Expression<'a>)> {
    let structured = !styles.is_empty()
        && matches!(
            crate::utils::unwrap_syntax_only(source),
            Expression::ArrayExpression(_)
        )
        && !crate::static_assignment::literal_source(source)
        || styles.iter().any(|style| {
            matches!(
                style,
                ExtractStyleProp::Conditional { .. } | ExtractStyleProp::MemberExpression { .. }
            )
        });
    let scalar = if structured && styles.iter().all(scalar_projection::base_only) {
        crate::gen_class_name::gen_class_names(ast, styles, None, None)
            .filter(|class| scalar_projection::preserves_reads(source, class))
    } else {
        None
    };
    if structured && scalar.is_none() {
        *styles = vec![ExtractStyleProp::Evaluated {
            binding: crate::sparse_sites::binding_name(source.span().start),
            styles: std::mem::take(styles),
            source: source.clone_in(ast.allocator()),
            evaluation: None,
            alternate_order: None,
            alternate_class: false,
        }];
        return None;
    }
    if let [ExtractStyleProp::Enum { condition, .. }] = styles.as_mut_slice() {
        let name = format!("__devupSpread{next}");
        *next += 1;
        let input = std::mem::replace(
            condition,
            Expression::new_identifier(
                source.span(),
                Str::from_in(name.as_str(), ast.allocator()),
                ast,
            ),
        );
        return Some((name, input));
    }
    let mut class =
        scalar.or_else(|| crate::gen_class_name::gen_class_names(ast, styles, None, None))?;
    if matches!(class, Expression::StringLiteral(_))
        && get_string_by_literal_expression(source).is_none()
    {
        class = crate::utils::call_with_values(
            ast,
            vec![(
                "__devupTypography".to_string(),
                source.clone_in(ast.allocator()),
            )],
            class,
        );
    }
    *class.span_mut() = source.span();
    let name = format!("__devupSpread{next}");
    *next += 1;
    let values = styles.iter().flat_map(ExtractStyleProp::extract).collect();
    *styles = vec![ExtractStyleProp::Expression {
        styles: values,
        expression: Expression::new_identifier(
            source.span(),
            Str::from_in(name.as_str(), ast.allocator()),
            ast,
        ),
    }];
    Some((name, class))
}
