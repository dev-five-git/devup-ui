use crate::ExtractStyleValue;
use crate::extract_style::extract_static_style::ExtractStaticStyle;

pub(super) const GETTERS: &str = "trace.push('built');const a=Card({get stop(){trace.push('get');return true}},null);const b=Card({get stop(){trace.push('get');return false}},null);";
pub(super) const PLAIN: &str = "trace.push('built');const a=Card({},null);const b=Card({},null);";

pub(super) fn fixture(body: &str, renders: &str) -> String {
    format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=render=>render;const Card=styled.div`{body}`;{renders}"
    )
}

pub(super) fn declarations(output: &crate::ExtractOutput) -> Vec<ExtractStyleValue> {
    let mut styles = output.styles.iter().cloned().collect::<Vec<_>>();
    styles.sort_unstable();
    styles
}

pub(super) fn red(orders: &[Option<u8>]) -> Vec<ExtractStyleValue> {
    let mut styles = orders
        .iter()
        .map(|order| {
            let mut style = ExtractStaticStyle::new("color", "red", 0, None);
            style.style_order = *order;
            ExtractStyleValue::Static(style)
        })
        .collect::<Vec<_>>();
    styles.sort_unstable();
    styles
}

pub(super) fn tokens(classes: &[String]) -> Vec<Vec<&str>> {
    classes
        .iter()
        .map(|class| class.split_whitespace().collect())
        .collect()
}
