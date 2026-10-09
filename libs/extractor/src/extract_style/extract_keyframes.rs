use std::collections::BTreeMap;

use css::{content_name::ContentName, style_origin::Origin};

use crate::extract_style::{
    ExtractStyleProperty, extract_static_style::ExtractStaticStyle, style_property::StyleProperty,
};

#[path = "keyframe_identity.rs"]
mod identity;

#[derive(Clone)]
pub struct ExtractKeyframes {
    pub keyframes: BTreeMap<String, Vec<ExtractStaticStyle>>,
    pub origin: Origin,
    pub(crate) producer_policy: super::ProducerPolicy,
}

impl Default for ExtractKeyframes {
    fn default() -> Self {
        Self {
            keyframes: BTreeMap::new(),
            origin: Origin::default(),
            producer_policy: crate::sparse_sites::producer_policy(),
        }
    }
}

impl std::fmt::Debug for ExtractKeyframes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractKeyframes")
            .field("keyframes", &self.keyframes)
            .finish()
    }
}

impl ExtractKeyframes {
    /// Produce one dormant keyframe allocation, never member allocations.
    ///
    /// # Errors
    /// Rejects Current construction on the parent or any child before reservation.
    pub fn counter_produce(
        &self,
        filename: Option<&str>,
    ) -> Result<super::ProducedAllocation, super::CounterProducerError> {
        super::counter_producer::produce_keyframes(self, filename)
    }

    /// The immutable identity policy selected when this record was constructed.
    #[must_use]
    pub const fn producer_policy(&self) -> super::ProducerPolicy {
        self.producer_policy
    }

    #[must_use]
    pub fn effective_steps(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.keyframes
            .iter()
            .map(|(step, styles)| {
                (
                    step.clone(),
                    styles
                        .iter()
                        .map(|style| (style.property().to_string(), style.effective_value()))
                        .collect(),
                )
            })
            .collect()
    }

    #[must_use]
    pub fn content_name(&self) -> ContentName {
        ContentName::keyframes(&self.effective_steps())
    }
}

impl ExtractStyleProperty for ExtractKeyframes {
    fn extract(&self, _filename: Option<&str>) -> StyleProperty {
        let content = self.content_name();
        StyleProperty::ClassName(content.name(css::get_prefix().as_deref().unwrap_or_default()))
    }
}
