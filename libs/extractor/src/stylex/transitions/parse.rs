use oxc_ast::ast::{Expression, ObjectExpression};
use oxc_span::GetSpan;

use super::declarations::{DeclarationContext, declarations, property, serialize};
use super::{DeclarationReader, Slot, TransitionApi, TransitionRules};
use crate::utils::{build_time_error, readable_code};

pub(super) fn rules(
    api: TransitionApi,
    object: &ObjectExpression<'_>,
    reader: &DeclarationReader<'_>,
) -> Result<TransitionRules, Vec<(u32, String)>> {
    match api {
        TransitionApi::Position => {
            let mut declarations =
                declarations(DeclarationContext { api, slot: None }, object, reader)?;
            declarations.sort_by(|(first, _), (second, _)| first.cmp(second));
            Ok(TransitionRules::Position(serialize(&declarations)))
        }
        TransitionApi::View => view(object, reader),
    }
}

fn view(
    object: &ObjectExpression<'_>,
    reader: &DeclarationReader<'_>,
) -> Result<TransitionRules, Vec<(u32, String)>> {
    let api = TransitionApi::View;
    let mut slots: Vec<(Slot, String)> = Vec::with_capacity(object.properties.len());
    let mut errors = vec![];
    for entry in &object.properties {
        let (key, property) = match property(api, entry) {
            Ok(property) => property,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let slot = match key.as_str() {
            "group" => Slot::Group,
            "imagePair" => Slot::ImagePair,
            "old" => Slot::Old,
            "new" => Slot::New,
            _ => {
                errors.push((
                    property.key.span().start,
                    build_time_error(
                        api.name(),
                        &key,
                        "its slots are group, imagePair, old and new",
                    ),
                ));
                continue;
            }
        };
        let Expression::ObjectExpression(object) = &property.value else {
            errors.push((
                property.value.span().start,
                build_time_error(
                    api.name(),
                    &readable_code(&property.value),
                    "each slot takes a flat object of static CSS declarations",
                ),
            ));
            continue;
        };
        let content = match declarations(
            DeclarationContext {
                api,
                slot: Some(slot),
            },
            object,
            reader,
        ) {
            Ok(declarations) => serialize(&declarations),
            Err(mut found) => {
                errors.append(&mut found);
                continue;
            }
        };
        match slots.iter_mut().find(|(previous, _)| *previous == slot) {
            Some((_, previous)) => *previous = content,
            None => slots.push((slot, content)),
        }
    }
    if errors.is_empty() {
        Ok(TransitionRules::View(slots))
    } else {
        Err(errors)
    }
}
