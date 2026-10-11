use css::style_selector::StyleSelector;

use super::DevupVisitor;
use crate::{ExtractStyleProp, ExtractStyleValue};

impl<'a> DevupVisitor<'a> {
    pub(super) fn publish_global_styles(&mut self, styles: Vec<ExtractStyleProp<'a>>, order: u8) {
        for mut style in styles.into_iter().flat_map(ExtractStyleProp::into_extract) {
            if let ExtractStyleValue::Static(style) = &mut style
                && let Some(
                    StyleSelector::Global(selector, _)
                    | StyleSelector::At {
                        selector: Some(selector),
                        file: Some(_),
                        ..
                    },
                ) = &mut style.selector
            {
                *selector = self.style_values.producer_selector(selector);
            }
            style.set_style_order(order);
            self.styles.insert(style);
        }
    }
}
