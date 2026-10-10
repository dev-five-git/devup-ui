use css::{Naming, content_name::AtomContent};
use sheet::StyleSheet;

#[path = "cache_reset_proof.rs"]
mod resets;

pub(crate) fn validate(sheet: &StyleSheet) -> Result<(), String> {
    if !resets::validate(sheet) {
        return Err("cached owner reset has no exact dynamic consumer proof".into());
    }
    crate::cache_special_proof::validate_frames(sheet)?;
    for orders in sheet.properties.values() {
        for (order, levels) in orders {
            for (level, properties) in levels {
                for property in properties {
                    let Some(claim) = sheet.names.get(&property.class_name) else {
                        continue;
                    };
                    if property.owner_reset {
                        continue;
                    }
                    if property.typography {
                        if crate::cache_special_proof::typography(
                            property,
                            (levels, *order),
                            &claim.descriptor,
                        )
                        .is_none()
                        {
                            return Err(format!(
                                "cached typography `{}` has a corrupt exact declaration proof",
                                property.class_name
                            ));
                        }
                        continue;
                    }
                    let valid = [Naming::Own, Naming::Risky].into_iter().any(|naming| {
                        [false, true].into_iter().any(|dynamic| {
                            let content = AtomContent {
                                property: &property.property,
                                value: Some(&property.value),
                                naming,
                                level: *level,
                                order: *order,
                                selector: property.selector.as_ref(),
                                layer: property.layer.as_deref(),
                                dynamic,
                            }
                            .content();
                            content.descriptor == claim.descriptor
                                && crate::cache_descriptor::name_matches(
                                    &content,
                                    &property.class_name,
                                )
                        })
                    });
                    if !valid {
                        return Err(format!(
                            "cached atom `{}` has a corrupt exact declaration proof",
                            property.class_name
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
