use super::Frame;
use css::{
    at_rule::{media_shorthand_query, split_at_rule_key},
    style_selector::{AtRuleKind, StyleSelector, is_selector_name},
    utils::to_kebab_case,
};

pub(crate) fn under(key: &str) -> Option<Frame> {
    if let Some((kind, query)) = split_at_rule_key(key) {
        return Some(Frame::At(kind, query.to_string()));
    }
    let name = key.strip_prefix('_').unwrap_or(key);
    let is_name = name
        .strip_prefix('-')
        .unwrap_or(name)
        .starts_with(|c: char| c.is_ascii_alphabetic())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if key.starts_with('_') || (!key.contains('&') && is_name) {
        let name = to_kebab_case(name);
        if let Some(query) = media_shorthand_query(&name) {
            return Some(Frame::At(AtRuleKind::Media, query.to_string()));
        }
        return is_selector_name(&name)
            .then(|| Frame::Selector(StyleSelector::from(name.as_ref()).to_string()));
    }
    Some(Frame::Selector(if key.contains('&') {
        key.to_string()
    } else if key.starts_with([':', '[']) {
        format!("&{key}")
    } else {
        format!("& {key}")
    }))
}
