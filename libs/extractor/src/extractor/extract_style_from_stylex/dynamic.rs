use super::{Leaf, raw_static_style};
use crate::ExtractStyleProp;
use crate::extract_style::extract_dynamic_style::ExtractDynamicStyle;
use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::stylex::assignments::is_final_assignment;
use crate::stylex::{DynamicNamespace, Scalar, dynamic_number_suffix, normalize_stylex_property};
use crate::utils::{
    build_time_error, get_string_by_property_key, key_error, readable_code, spread_error,
};
use css::sheet_to_variable_name;
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, Expression, ObjectPropertyKind, PropertyKind,
};
use oxc_span::GetSpan;

/// Validate the function before extracting its scalar property assignments.
pub(super) fn extract<'a>(
    arrow: &ArrowFunctionExpression<'_>,
    leaf: &Leaf<'_>,
    errors: &mut Vec<(u32, String)>,
) -> Option<(Vec<ExtractStyleProp<'a>>, DynamicNamespace)> {
    let allocator = oxc_allocator::Allocator::default();
    let arrow_code = readable_code(&Expression::ArrowFunctionExpression(
        oxc_allocator::Box::new_in(arrow.clone_in(&allocator), &&allocator),
    ));
    let mut reject = |offset, form: &str| {
        errors.push((offset, build_time_error("stylex.create", &arrow_code,
            &format!("a dynamic style is not exact ({form}); this function form cannot be compiled exactly; use a synchronous expression-bodied arrow with plain scalar parameters and literal defaults"))));
    };
    if arrow.r#async {
        reject(arrow.span.start, "async arrow function");
        return None;
    }
    if let Some(rest) = &arrow.params.rest {
        reject(rest.span.start, "rest parameter");
        return None;
    }
    let mut names = vec![];
    let mut namespace = DynamicNamespace::default();
    for param in &arrow.params.items {
        let BindingPattern::BindingIdentifier(ident) = &param.pattern else {
            reject(
                param.pattern.span().start,
                "destructuring parameter/default",
            );
            return None;
        };
        let default = match &param.initializer {
            Some(value) => {
                if let Some(default) = Scalar::literal(value) {
                    Some(default)
                } else {
                    reject(
                        value.span().start,
                        &format!("non-exact default `{}`", readable_code(value)),
                    );
                    return None;
                }
            }
            None => None,
        };
        names.push(ident.name.to_string());
        namespace.defaults.push(default);
    }
    let Some(body) = arrow.body.as_expression() else {
        reject(arrow.body.span().start, "block/statement body");
        return None;
    };
    let Expression::ObjectExpression(body) = body.without_parentheses() else {
        reject(
            body.span().start,
            &format!("non-object body `{}`", readable_code(body)),
        );
        return None;
    };
    let mut styles = vec![];
    for (position, prop) in body.properties.iter().enumerate() {
        let prop = match prop {
            ObjectPropertyKind::ObjectProperty(prop) => prop,
            ObjectPropertyKind::SpreadProperty(spread) => {
                errors.push(spread_error("stylex.create", spread));
                continue;
            }
        };
        if prop.method || prop.kind != PropertyKind::Init {
            errors.push((
                prop.span.start,
                build_time_error(
                    "stylex.create",
                    "method/getter/setter body property",
                    "use a plain scalar property assignment",
                ),
            ));
            continue;
        }
        let Some(name) = get_string_by_property_key(&prop.key) else {
            errors.push(key_error("stylex.create", &prop.key));
            continue;
        };
        let property = normalize_stylex_property(&name);
        let final_assignment = is_final_assignment(&name, &body.properties[position + 1..]);
        if final_assignment {
            namespace.properties.push(property.clone());
        }
        let index = match prop.value.without_parentheses() {
            Expression::Identifier(ident) => {
                names.iter().position(|name| name == ident.name.as_str())
            }
            _ => None,
        };
        if let Some(index) = index {
            if !final_assignment {
                continue;
            }
            namespace.css_vars.push((
                index,
                sheet_to_variable_name(&property, 0, None),
                dynamic_number_suffix(&property),
                property.clone(),
            ));
            styles.push(ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                ExtractDynamicStyle::new(&property, 0, &names[index], None),
            )));
        } else if !matches!(prop.value.without_parentheses(), Expression::NullLiteral(_)) {
            if let Some(value) = leaf(&property, &prop.value) {
                if final_assignment {
                    styles.push(raw_static_style(property, &value, None));
                }
            } else {
                errors.push((prop.value.span().start, build_time_error("stylex.create", &readable_code(&prop.value), "a dynamic style's value is a non-exact body value; use a parameter or a static scalar value; compute scalar expressions before passing them")));
            }
        }
    }
    Some((styles, namespace))
}
