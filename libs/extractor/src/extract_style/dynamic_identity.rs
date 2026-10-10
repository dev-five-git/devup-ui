use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use css::{Naming, Site, style_origin::Origin, style_selector::StyleSelector};

use super::ExtractDynamicStyle;
use crate::extract_style::ProducerPolicy;
use crate::extract_style::counter_selector::CounterSelector;
use crate::extract_style::numeric_conversion::NumericConversion;

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Identity<'a> {
    property: &'a str,
    level: u8,
    identifier: &'a str,
    selector: &'a Option<StyleSelector>,
    order: Option<u8>,
    important: bool,
    layer: &'a Option<String>,
    naming: Naming,
    site: &'a Option<Site>,
    origin: &'a Origin,
    conversion: NumericConversion,
    presence: bool,
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CounterIdentity<'a> {
    property: &'a str,
    level: u8,
    identifier: &'a str,
    selector: CounterSelector<'a>,
    order: Option<u8>,
    important: bool,
    layer: &'a Option<String>,
    site: &'a Option<Site>,
    owner: Option<u32>,
    conversion: NumericConversion,
    presence: bool,
}

impl ExtractDynamicStyle {
    fn identity(&self) -> Identity<'_> {
        let ExtractDynamicStyle {
            property,
            level,
            identifier,
            conversion,
            presence,
            selector,
            style_order,
            important,
            layer,
            naming,
            site,
            origin,
            producer_policy: _,
        } = self;
        Identity {
            property,
            level: *level,
            identifier,
            selector,
            order: *style_order,
            important: *important,
            layer,
            naming: *naming,
            site,
            origin,
            conversion: *conversion,
            presence: *presence,
        }
    }

    fn counter_identity(&self, original: u32) -> CounterIdentity<'_> {
        let ExtractDynamicStyle {
            property,
            level,
            identifier,
            conversion,
            presence,
            selector,
            style_order,
            important,
            layer,
            naming: _,
            site,
            origin: _,
            producer_policy: _,
        } = self;
        CounterIdentity {
            property,
            level: *level,
            identifier,
            selector: CounterSelector(selector),
            order: *style_order,
            important: *important,
            layer,
            site,
            owner: ProducerPolicy::declaration_owner(original, *style_order),
            conversion: *conversion,
            presence: *presence,
        }
    }
}

impl PartialEq for ExtractDynamicStyle {
    fn eq(&self, other: &Self) -> bool {
        match (self.producer_policy(), other.producer_policy()) {
            (ProducerPolicy::Current, ProducerPolicy::Current) => {
                self.identity() == other.identity()
            }
            (ProducerPolicy::Current, ProducerPolicy::CounterOriginal(_))
            | (ProducerPolicy::CounterOriginal(_), ProducerPolicy::Current) => false,
            (ProducerPolicy::CounterOriginal(left), ProducerPolicy::CounterOriginal(right)) => {
                self.counter_identity(left) == other.counter_identity(right)
            }
        }
    }
}
impl Eq for ExtractDynamicStyle {}
impl Hash for ExtractDynamicStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.producer_policy() {
            ProducerPolicy::Current => self.identity().hash(state),
            ProducerPolicy::CounterOriginal(original) => {
                1u8.hash(state);
                self.counter_identity(original).hash(state);
            }
        }
    }
}
impl PartialOrd for ExtractDynamicStyle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ExtractDynamicStyle {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.producer_policy(), other.producer_policy()) {
            (ProducerPolicy::Current, ProducerPolicy::Current) => {
                self.identity().cmp(&other.identity())
            }
            (ProducerPolicy::Current, ProducerPolicy::CounterOriginal(_)) => Ordering::Less,
            (ProducerPolicy::CounterOriginal(_), ProducerPolicy::Current) => Ordering::Greater,
            (ProducerPolicy::CounterOriginal(left), ProducerPolicy::CounterOriginal(right)) => self
                .counter_identity(left)
                .cmp(&other.counter_identity(right)),
        }
    }
}
