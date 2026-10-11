use sheet::StyleSheet;

pub(super) fn validate(sheet: &StyleSheet) -> bool {
    sheet.properties.values().all(|orders| {
        orders.values().all(|levels| {
            levels.iter().all(|(level, properties)| {
                properties
                    .iter()
                    .filter(|property| property.owner_reset)
                    .all(|reset| {
                        *level == 0
                            && reset.value == "initial"
                            && reset.property.starts_with("--")
                            && reset.selector.is_none()
                            && reset.layer.is_none()
                            && !reset.typography
                            && levels.values().flatten().any(|consumer| {
                                !consumer.owner_reset
                                    && !consumer.typography
                                    && consumer.class_name == reset.class_name
                                    && consumer.hoisted == reset.hoisted
                                    && consumer.value.strip_prefix("var(").and_then(|value| {
                                        value
                                            .strip_suffix(')')
                                            .or_else(|| value.strip_suffix(") !important"))
                                    }) == Some(reset.property.as_str())
                                    && sheet
                                        .names
                                        .get(&consumer.class_name)
                                        .is_none_or(|claim| claim.descriptor.starts_with(&[1, 2]))
                            })
                    })
            })
        })
    })
}
