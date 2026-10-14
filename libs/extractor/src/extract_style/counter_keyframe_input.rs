use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use css::allocation_input::NameMode;

use super::{ExtractKeyframes, extract_static_style::ExtractStaticStyle};

pub(super) struct LegacyMember<'a>(pub(super) &'a ExtractStaticStyle);

impl Hash for LegacyMember<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let style = self.0;
        style.property.hash(state);
        style.value.hash(state);
        style.level.hash(state);
        style.selector.hash(state);
        style.style_order.hash(state);
        style.layer.hash(state);
        style.theme_token_resolution.hash(state);
    }
}

pub(super) fn legacy_map(frames: &ExtractKeyframes) -> BTreeMap<&String, Vec<LegacyMember<'_>>> {
    frames
        .keyframes
        .iter()
        .map(|(step, styles)| (step, styles.iter().map(LegacyMember).collect()))
        .collect()
}

pub(super) fn input(frames: &ExtractKeyframes, mode: NameMode) -> String {
    match mode {
        NameMode::Counter | NameMode::Debug => {
            let mut hasher = DefaultHasher::new();
            legacy_map(frames).hash(&mut hasher);
            hasher.finish().to_string()
        }
        NameMode::AtomHoist => {
            let mut content = String::new();
            for (step, styles) in &frames.keyframes {
                content.push_str(&css::atom_name::hex(step));
                content.push('{');
                for style in styles {
                    content.push_str(&css::atom_name::hex(style.property()));
                    content.push(':');
                    content.push_str(&css::atom_name::hex(style.value()));
                    content.push(';');
                }
                content.push('}');
            }
            content
        }
    }
}
