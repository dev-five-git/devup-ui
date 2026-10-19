use crate::style_selector::StyleSelector;

/// The bare functional pseudo targeted by `add_selector_params`, without allocating.
#[must_use]
pub fn parameter_name(selector: &StyleSelector) -> Option<&str> {
    let value = match selector {
        StyleSelector::Selector(value)
        | StyleSelector::Global(value, _)
        | StyleSelector::At {
            selector: Some(value),
            ..
        } => value,
        StyleSelector::At { selector: None, .. } => return None,
    };
    let end = crate::params_position(value)?;
    value[..end].rsplit(':').next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("&:nth-child", Some("nth-child"))]
    #[case(":is([role=group],[data-group]):nth-child &:hover", Some("nth-child"))]
    #[case("&:nth-child(2):hover", None)]
    #[case("&:hover", None)]
    #[case("&::slotted", Some("slotted"))]
    fn finds_the_same_bare_target_as_the_formatter(
        #[case] selector: &str,
        #[case] expected: Option<&str>,
    ) {
        let selector = StyleSelector::Selector(selector.to_string());
        assert_eq!(parameter_name(&selector), expected);
    }
}
