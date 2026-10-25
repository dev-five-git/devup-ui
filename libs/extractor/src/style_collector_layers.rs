use super::StyleCollector;
use crate::{ExtractStyleProp, extract_style::extract_style_value::ExtractStyleValue};

impl StyleCollector {
    /// Register authored layers before composition can replace their first atoms.
    pub(crate) fn use_layers(&mut self, props: &[ExtractStyleProp<'_>]) {
        self.scan_layers(props, true);
        self.scan_layers(props, false);
    }

    fn scan_layers(&mut self, props: &[ExtractStyleProp<'_>], registrations: bool) {
        let mut pending: Vec<_> = props.iter().rev().collect();
        while let Some(prop) = pending.pop() {
            let values = match prop {
                ExtractStyleProp::Static(value) => std::slice::from_ref(value),
                ExtractStyleProp::Expression { styles, .. } => styles.as_slice(),
                ExtractStyleProp::StaticArray(props) => {
                    pending.extend(props.iter().rev());
                    continue;
                }
                ExtractStyleProp::Conditional {
                    consequent,
                    alternate,
                    ..
                } => {
                    pending.extend(
                        [alternate, consequent]
                            .into_iter()
                            .filter_map(|side| side.as_deref()),
                    );
                    continue;
                }
                ExtractStyleProp::Enum { map, .. } => {
                    pending.extend(map.values().rev().flat_map(|props| props.iter().rev()));
                    continue;
                }
                ExtractStyleProp::MemberExpression { map, .. } => {
                    pending.extend(map.values().rev().map(AsRef::as_ref));
                    continue;
                }
                ExtractStyleProp::Unreadable { .. } => continue,
            };
            for value in values {
                match value {
                    ExtractStyleValue::LayerOrder(order) if registrations => {
                        for layer in &order.layers {
                            self.use_layer(Some(layer));
                        }
                    }
                    ExtractStyleValue::Static(style) if !registrations => {
                        self.use_layer(style.layer());
                    }
                    ExtractStyleValue::Dynamic(style) if !registrations => {
                        self.use_layer(style.layer());
                    }
                    ExtractStyleValue::Static(_)
                    | ExtractStyleValue::Dynamic(_)
                    | ExtractStyleValue::Css(_)
                    | ExtractStyleValue::Import(_)
                    | ExtractStyleValue::FontFace(_)
                    | ExtractStyleValue::Typography(_)
                    | ExtractStyleValue::Keyframes(_)
                    | ExtractStyleValue::LayerOrder(_) => {}
                }
            }
        }
    }
}
