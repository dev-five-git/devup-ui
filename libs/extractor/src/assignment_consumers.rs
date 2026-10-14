use crate::{ExtractStyleProp, utils::expression_to_code};
use oxc_span::GetSpan;

pub(crate) fn merge(styles: &mut Vec<ExtractStyleProp<'_>>) {
    let mut result = Vec::new();
    for style in std::mem::take(styles) {
        match style {
            ExtractStyleProp::StaticArray(mut inner) => {
                merge(&mut inner);
                result.extend(inner);
            }
            style => result.push(style),
        }
    }
    for style in result {
        let existing = styles
            .iter_mut()
            .find(|existing| match (&style, &**existing) {
                (
                    ExtractStyleProp::Conditional {
                        condition: left, ..
                    },
                    ExtractStyleProp::Conditional {
                        condition: right, ..
                    },
                )
                | (
                    ExtractStyleProp::MemberExpression {
                        expression: left, ..
                    },
                    ExtractStyleProp::MemberExpression {
                        expression: right, ..
                    },
                ) => {
                    left.span() == right.span()
                        && expression_to_code(left) == expression_to_code(right)
                }
                _ => false,
            });
        match (existing, style) {
            (
                Some(ExtractStyleProp::Conditional {
                    consequent,
                    alternate,
                    ..
                }),
                ExtractStyleProp::Conditional {
                    consequent: added_con,
                    alternate: added_alt,
                    ..
                },
            ) => {
                append_branch(consequent, added_con);
                append_branch(alternate, added_alt);
            }
            (
                Some(ExtractStyleProp::MemberExpression { map, .. }),
                ExtractStyleProp::MemberExpression { map: added, .. },
            ) => {
                for (key, added) in added {
                    let selected = map
                        .entry(key)
                        .or_insert_with(|| Box::new(ExtractStyleProp::StaticArray(vec![])));
                    let original =
                        std::mem::replace(selected.as_mut(), ExtractStyleProp::StaticArray(vec![]));
                    let mut consumers = vec![original, *added];
                    merge(&mut consumers);
                    **selected = ExtractStyleProp::StaticArray(consumers);
                }
            }
            (_, style) => styles.push(style),
        }
    }
}

fn append_branch<'a>(
    branch: &mut Option<Box<ExtractStyleProp<'a>>>,
    added: Option<Box<ExtractStyleProp<'a>>>,
) {
    if let Some(added) = added {
        let mut styles = branch
            .take()
            .into_iter()
            .map(|style| *style)
            .chain([*added])
            .collect();
        merge(&mut styles);
        *branch = Some(Box::new(ExtractStyleProp::StaticArray(styles)));
    }
}
