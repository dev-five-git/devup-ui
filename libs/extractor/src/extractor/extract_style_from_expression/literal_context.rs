use crate::ExtractStyleProp;
use oxc_ast::ast::Expression;

#[cfg(test)]
pub(super) fn reject<'a>(
    name: Option<&str>,
    value: &Expression<'a>,
) -> Option<Vec<ExtractStyleProp<'a>>> {
    reject_payload(name, value)
}

pub(super) fn reject_payload<'a, E>(
    name: Option<&str>,
    value: &Expression<'a>,
) -> Option<Vec<ExtractStyleProp<'a, E>>> {
    let name = name?;
    let api = match name.split_whitespace().next()? {
        "@font-face" => "fontFaces",
        "@keyframes" | "@-webkit-keyframes" => "keyframes",
        _ => return None,
    };
    let mut errors = Vec::new();
    crate::style_order::reject(value, api, &mut errors);
    if errors.is_empty() {
        return None;
    }
    Some(
        errors
            .into_iter()
            .map(|(offset, message)| ExtractStyleProp::Diagnostic {
                offset,
                message,
                disposition: crate::ErrorDisposition::Definitive,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_audit_when_descriptor_has_no_metadata_leaves_existing_policy_intact() {
        let allocator = oxc_allocator::Allocator::default();
        let parsed = oxc_parser::Parser::new(
            &allocator,
            "({fontFamily:'Test'});",
            oxc_span::SourceType::ts(),
        )
        .parse();
        let oxc_ast::ast::Statement::ExpressionStatement(statement) = &parsed.program.body[0]
        else {
            panic!("fixture must contain a descriptor expression");
        };
        assert!(reject(Some("@font-face"), &statement.expression).is_none());
    }
}
