use css::style_selector::StyleSelector;

use super::DevupVisitor;
use crate::{ExtractStyleProp, ExtractStyleValue};

impl<'a> DevupVisitor<'a> {
    pub(super) fn publish_global_styles(&mut self, styles: Vec<ExtractStyleProp<'a>>, order: u8) {
        for mut style in styles.into_iter().flat_map(ExtractStyleProp::into_extract) {
            match &mut style {
                ExtractStyleValue::Static(style) => match &mut style.selector {
                    Some(
                        StyleSelector::Global(selector, _)
                        | StyleSelector::At {
                            selector: Some(selector),
                            file: Some(_),
                            ..
                        },
                    ) => {
                        *selector = self.style_values.producer_selector(selector);
                    }
                    Some(StyleSelector::Selector(_) | StyleSelector::At { .. }) | None => {}
                },
                ExtractStyleValue::Dynamic(_)
                | ExtractStyleValue::Typography(_)
                | ExtractStyleValue::Css(_)
                | ExtractStyleValue::Import(_)
                | ExtractStyleValue::FontFace(_)
                | ExtractStyleValue::Keyframes(_) => {}
            }
            style.set_style_order(order);
            self.styles.insert(style);
        }
    }
}
