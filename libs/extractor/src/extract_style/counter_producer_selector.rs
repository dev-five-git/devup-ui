use css::{allocation_input::NameMode, style_selector::StyleSelector};

/// Project the baseline selector/layer key using only the already captured mode.
pub(super) fn class_selector(
    selector: Option<&StyleSelector>,
    layer: Option<&str>,
    mode: NameMode,
) -> Option<String> {
    match mode {
        NameMode::AtomHoist => Some(css::atom_name::selector_key(selector, layer)),
        NameMode::Counter | NameMode::Debug => {
            let selector = selector.map(|selector| selector.as_class_str().into_owned());
            match layer {
                Some(layer) => Some(format!(
                    "{}@layer {layer}",
                    selector.as_deref().unwrap_or_default()
                )),
                None => selector,
            }
        }
    }
}
