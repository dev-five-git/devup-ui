use css::style_selector::StyleSelector;
use rustc_hash::FxHashSet;

use crate::extract_style::{
    extract_layer_order::ExtractLayerOrder, extract_style_value::ExtractStyleValue,
};

/// The styles a file writes. Global rules are numbered in the order the file
/// writes them, since that order decides which of them wins, and layers are
/// listed in the order the file first uses them, since that order decides
/// which layer wins.
#[derive(Default)]
pub struct StyleCollector {
    styles: FxHashSet<ExtractStyleValue>,
    /// The number the last global rule got; `0` stays for styles without one
    written: u32,
    layers: Vec<String>,
}

impl StyleCollector {
    pub fn insert(&mut self, mut style: ExtractStyleValue) {
        match &mut style {
            ExtractStyleValue::Css(css) => css.order = self.next_order(),
            ExtractStyleValue::Import(import) => import.order = self.next_order(),
            ExtractStyleValue::FontFace(font_face) => font_face.order = self.next_order(),
            ExtractStyleValue::Static(style) => {
                if matches!(style.selector, Some(StyleSelector::Global(..))) {
                    style.order = self.next_order();
                }
                self.use_layer(style.layer.as_deref());
            }
            ExtractStyleValue::Dynamic(style) => self.use_layer(style.layer()),
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Keyframes(_)
            | ExtractStyleValue::LayerOrder(_) => {}
        }
        self.styles.insert(style);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    /// The styles of `file`, with the order of its layers when it uses any
    #[must_use]
    pub fn into_styles(mut self, file: &str) -> FxHashSet<ExtractStyleValue> {
        if !self.layers.is_empty() {
            self.styles
                .insert(ExtractStyleValue::LayerOrder(ExtractLayerOrder {
                    file: file.to_string(),
                    layers: self.layers,
                }));
        }
        self.styles
    }

    const fn next_order(&mut self) -> u32 {
        self.written += 1;
        self.written
    }

    /// A nested layer comes after the layers it sits in, as CSS declares them
    fn use_layer(&mut self, layer: Option<&str>) {
        let Some(layer) = layer else {
            return;
        };
        let ends = layer
            .match_indices('.')
            .map(|(index, _)| index)
            .chain([layer.len()]);
        for end in ends {
            let name = &layer[..end];
            if !self.layers.iter().any(|used| used == name) {
                self.layers.push(name.to_string());
            }
        }
    }
}

impl Extend<ExtractStyleValue> for StyleCollector {
    fn extend<I: IntoIterator<Item = ExtractStyleValue>>(&mut self, styles: I) {
        for style in styles {
            self.insert(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_style::{
        extract_css::ExtractCss, extract_dynamic_style::ExtractDynamicStyle,
        extract_font_face::ExtractFontFace, extract_import::ExtractImport,
        extract_keyframes::ExtractKeyframes, extract_static_style::ExtractStaticStyle,
    };

    fn global(property: &str, value: &str) -> ExtractStyleValue {
        ExtractStyleValue::Static(ExtractStaticStyle::new(
            property,
            value,
            0,
            Some(StyleSelector::Global(
                "body".to_string(),
                "a.tsx".to_string(),
            )),
        ))
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn numbers_global_rules_in_written_order_and_lists_layers() {
        let mut collector = StyleCollector::default();
        assert!(collector.is_empty());
        collector.extend([
            ExtractStyleValue::Import(ExtractImport {
                file: "a.tsx".to_string(),
                order: 0,
                url: "z.css".to_string(),
            }),
            global("color", "red"),
            ExtractStyleValue::Css(ExtractCss {
                file: "a.tsx".to_string(),
                order: 0,
                css: "body{color:blue}".to_string(),
            }),
            ExtractStyleValue::FontFace(ExtractFontFace {
                file: "a.tsx".to_string(),
                order: 0,
                properties: Default::default(),
            }),
            ExtractStyleValue::Static(ExtractStaticStyle::new_with_layer(
                "color",
                "red",
                0,
                None,
                Some("reset".to_string()),
            )),
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "--a", None)),
            ExtractStyleValue::Typography("body".to_string()),
            ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
            global("color", "blue"),
        ]);
        for property in ["margin", "padding"] {
            let mut layered = ExtractDynamicStyle::new(property, 0, "--b", None);
            layered.layer = Some("base.inner".to_string());
            collector.insert(ExtractStyleValue::Dynamic(layered));
        }
        assert!(!collector.is_empty());

        let styles = collector.into_styles("a.tsx");
        let mut orders: Vec<(u32, String)> = styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Import(import) => Some((import.order, import.url.clone())),
                ExtractStyleValue::Css(css) => Some((css.order, css.css.clone())),
                ExtractStyleValue::FontFace(font_face) => {
                    Some((font_face.order, "@font-face".to_string()))
                }
                ExtractStyleValue::Static(style) => Some((
                    style.order(),
                    format!("{}:{}", style.property(), style.value()),
                )),
                _ => None,
            })
            .collect();
        orders.sort();
        assert_eq!(
            orders,
            [
                (0, "color:red".to_string()),
                (1, "z.css".to_string()),
                (2, "color:red".to_string()),
                (3, "body{color:blue}".to_string()),
                (4, "@font-face".to_string()),
                (5, "color:blue".to_string()),
            ]
        );
        assert!(
            styles.contains(&ExtractStyleValue::LayerOrder(ExtractLayerOrder {
                file: "a.tsx".to_string(),
                layers: vec![
                    "reset".to_string(),
                    "base".to_string(),
                    "base.inner".to_string()
                ],
            }))
        );
    }

    #[test]
    fn leaves_the_layer_order_out_without_layers() {
        let mut collector = StyleCollector::default();
        collector.insert(global("color", "red"));
        let styles = collector.into_styles("a.tsx");
        assert_eq!(styles.len(), 1);
    }
}
