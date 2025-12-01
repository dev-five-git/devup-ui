use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use css::keyframes_to_keyframes_name;

use crate::extract_style::{ExtractStyleProperty, extract_static_style::ExtractStaticStyle, style_property::StyleProperty};

#[derive(Debug, Default, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractKeyframes {
    pub keyframes: BTreeMap<String, Vec<ExtractStaticStyle>>,
}

impl ExtractStyleProperty for ExtractKeyframes {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let mut hasher = DefaultHasher::new();
        self.keyframes.hash(&mut hasher);
        let hash_key = hasher.finish().to_string();
        StyleProperty::ClassName(keyframes_to_keyframes_name(&hash_key, filename))
    }
}
