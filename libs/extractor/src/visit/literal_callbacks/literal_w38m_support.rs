use crate::ExtractStyleValue;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extract_style::style_property::StyleProperty;
use css::style_selector::StyleSelector;

pub(super) const GETTERS: &str = "trace.push('built');const a=Card({get stop(){trace.push('get');return true}},null);const b=Card({get stop(){trace.push('get');return false}},null);";

pub(super) fn fixture(root: &str, renders: &str) -> String {
    format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=render=>render;const rest={{color:'red'}};const Card=styled('div',{{{root}}});{renders}"
    )
}

pub(super) fn color(value: &str, selector: Option<&str>, order: Option<u8>) -> ExtractStyleValue {
    let selector = selector.map(|selector| StyleSelector::Selector(selector.to_string()));
    let mut style = ExtractStaticStyle::new("color", value, 0, selector);
    style.style_order = order;
    ExtractStyleValue::Static(style)
}

pub(super) fn rules(root: &str, orders: &[Option<u8>]) -> Vec<ExtractStyleValue> {
    let mut expected = vec![color(root, None, None)];
    expected.extend(
        orders
            .iter()
            .map(|order| color("blue", Some("&:hover"), *order)),
    );
    expected.sort_unstable();
    expected
}

pub(super) fn selected(root: &str, orders: &[Option<u8>]) -> Vec<Vec<String>> {
    orders
        .iter()
        .map(|order| {
            class_tokens(&[
                color(root, None, None),
                color("blue", Some("&:hover"), *order),
            ])
        })
        .collect()
}

pub(super) fn class_tokens(styles: &[ExtractStyleValue]) -> Vec<String> {
    css::debug::set_debug(true);
    let selected = styles
        .iter()
        .map(|style| match style.extract(Some("a.tsx")) {
            Some(StyleProperty::ClassName(class)) => class,
            Some(StyleProperty::Variable { .. }) | None => panic!("static fixture token"),
        })
        .collect();
    css::debug::set_debug(false);
    selected
}

pub(super) fn located(source: &str, token: &str) -> String {
    let offset = source
        .find(token)
        .unwrap_or_else(|| panic!("fixture token: {token}"));
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_else(|| panic!("line"))
        .len()
        + 1;
    format!("a.tsx:{line}:{column}:")
}

pub(super) fn policy_error(source: &str, operand: &str, occurrence: usize) -> String {
    let (offset, _) = source
        .match_indices(operand)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("source operand: {operand}"));
    let code = operand.replace('\'', "\"");
    let message = crate::utils::build_time_error("styled", &code, crate::utils::STYLE_OBJECT);
    format!("a.tsx:1:{}: {message}", offset + 1)
}
