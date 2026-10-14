use super::{Candidate, error::KernelError};
use crate::{counter_evidence::AllocationEvidence, emission_seed::EmissionContext, theme::Theme};
use css::allocation_input::CapturedDelivery;
use extractor::extract_style::{
    CounterProducerError, ExtractDynamicStyle, ExtractKeyframes, ProducedAllocation,
    ProducerPolicy, extract_static_style::ExtractStaticStyle,
};

#[path = "counter_kernel_receipt_application.rs"]
pub(in crate::counter_kernel) mod application;
use application::{Acquisition, dynamic_core, static_core};
use extractor::extract_style::extract_style_value::ExtractStyleValue as Value;

impl<'a> From<&'a rustc_hash::FxHashSet<Value>> for super::prepare::Input<'a> {
    fn from(styles: &'a rustc_hash::FxHashSet<Value>) -> Self {
        Self::Constructor(styles)
    }
}

impl<'a> Acquisition<'a> {
    pub(in crate::counter_kernel) const fn class(&self) -> &'a ProducedAllocation {
        match self {
            Self::Static(_, receipt) | Self::Keyframes(_, receipt) => receipt,
            Self::Dynamic(_, receipt) => &receipt.class,
        }
    }
}

/// Full raw equality; semantic cores deliberately exclude transition dimensions.
pub(super) fn same(a: &Value, b: &Value) -> bool {
    same_request(a, b, false)
}

pub(super) fn same_request(a: &Value, b: &Value, ordered: bool) -> bool {
    match (a, b) {
        (Value::Static(a), Value::Static(b)) => {
            static_core(a) == static_core(b) && (ordered || a.style_order() == b.style_order())
        }
        (Value::Dynamic(a), Value::Dynamic(b)) => {
            dynamic_core(a) == dynamic_core(b)
                && (ordered || a.style_order() == b.style_order())
                && a.identifier() == b.identifier()
        }
        (Value::Keyframes(a), Value::Keyframes(b)) => {
            a.producer_policy() == b.producer_policy()
                && a.keyframes.len() == b.keyframes.len()
                && a.keyframes
                    .iter()
                    .zip(&b.keyframes)
                    .all(|((ak, av), (bk, bv))| {
                        ak == bk
                            && av.len() == bv.len()
                            && av.iter().zip(bv).all(|(a, b)| {
                                static_core(a) == static_core(b)
                                    && a.style_order() == b.style_order()
                            })
                    })
        }
        (
            Value::Static(_)
            | Value::Dynamic(_)
            | Value::Keyframes(_)
            | Value::Typography(_)
            | Value::Css(_)
            | Value::Import(_)
            | Value::FontFace(_),
            _,
        ) => false,
    }
}

pub(super) fn require(valid: bool) -> Result<(), KernelError> {
    valid.then_some(()).ok_or(KernelError::Authority)
}

pub(super) const fn original(policy: ProducerPolicy) -> Result<u32, KernelError> {
    match policy {
        ProducerPolicy::CounterOriginal(original) => Ok(original),
        ProducerPolicy::Current => Err(KernelError::Producer(CounterProducerError::WrongPolicy)),
    }
}

pub(super) enum Generated<'a> {
    Static(&'a ExtractStaticStyle),
    Dynamic(&'a ExtractDynamicStyle),
    Keyframes(&'a ExtractKeyframes),
}

pub(super) fn allocation(receipt: ProducedAllocation) -> AllocationEvidence {
    AllocationEvidence {
        input: receipt.input,
        context: receipt.context,
        allocation: receipt.allocation,
    }
}

pub(super) fn candidate(
    style: Generated<'_>,
    theme: &Theme,
    placement: &EmissionContext,
) -> Result<(Candidate, CapturedDelivery), KernelError> {
    let filename = (!placement.single_css).then_some(placement.source_file.as_str());
    match style {
        Generated::Static(style) => {
            let receipt = style.counter_produce(filename)?;
            application::producer(Acquisition::Static(style, &receipt), theme, placement)
        }
        Generated::Dynamic(style) => {
            let receipt = style.counter_produce(filename)?;
            application::producer(Acquisition::Dynamic(style, &receipt), theme, placement)
        }
        Generated::Keyframes(style) => {
            let receipt = style.counter_produce(filename)?;
            application::producer(Acquisition::Keyframes(style, &receipt), theme, placement)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter_kernel::{FrozenAuthority, authentic_support::*, link};

    fn shared() -> EmissionContext {
        application::placement("a", None)
    }

    #[derive(Clone, Copy)]
    enum Damage {
        ClassOriginal,
        VariableOriginal,
        Identifier,
        Importance,
        Site,
    }

    #[rstest::rstest]
    #[case(Damage::ClassOriginal)]
    #[case(Damage::VariableOriginal)]
    #[case(Damage::Identifier)]
    #[case(Damage::Importance)]
    #[case(Damage::Site)]
    #[serial_test::serial]
    fn producer_rejects_damage_to_real_dynamic_acquisition(#[case] damage: Damage) {
        // Given
        let _state = state();
        let style = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
        let mut acquired = style.counter_produce(None).required("real acquisition");
        match damage {
            Damage::ClassOriginal => acquired.class.original = u32::MAX,
            Damage::VariableOriginal => {
                let variable = acquired.variable_allocation.as_mut().required("variable");
                variable.original = u32::MAX;
            }
            Damage::Identifier => acquired.identifier = "other".into(),
            Damage::Importance => acquired.important = !style.important(),
            Damage::Site => {
                acquired.site = Some(css::Site {
                    file: css::sparse_site::SourceFile::D9(0),
                    at: 1,
                    role: 0,
                });
            }
        }
        // When
        let result = application::producer(
            Acquisition::Dynamic(&style, &acquired),
            &Theme::default(),
            &shared(),
        );
        // Then
        assert_eq!(result, Err(KernelError::Authority));
    }

    #[test]
    #[serial_test::serial]
    fn independent_alias_keeps_its_original_without_counterfeit_acquisition() {
        // Given
        let _state = state();
        let producer = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
        let alias = fixture("b", || ExtractDynamicStyle::new("color", 0, "tone", None));
        let acquired = producer.counter_produce(None).required("real acquisition");
        // When
        let (candidate, delivery) = application::apply(
            Acquisition::Dynamic(&alias, &acquired),
            &Theme::default(),
            &shared(),
        )
        .required("independent alias");
        // Then
        assert_eq!(acquired.class.original, 0);
        assert_eq!(candidate.lineage.parent, 1);
        let variable = candidate.lineage.variable.as_ref().required("variable");
        assert_eq!(variable.original, 1);
        let mut authority = FrozenAuthority::live();
        authority.capture(&candidate, delivery).required("capture");
        assert_eq!(link::check(&candidate, &authority), Ok(()));
    }

    #[rstest::rstest]
    #[case(false)]
    #[case(true)]
    #[serial_test::serial]
    fn producer_rejects_static_or_keyframe_original_mismatch(#[case] keyframes: bool) {
        // Given
        let _state = state();
        let style = fixture("a", || ExtractStaticStyle::new("color", "red", 0, None));
        let frames = fixture("a", ExtractKeyframes::default);
        let mut acquired = if keyframes {
            frames.counter_produce(None).required("receipt")
        } else {
            style.counter_produce(None).required("static acquisition")
        };
        acquired.original = u32::MAX;
        let acquisition = if keyframes {
            Acquisition::Keyframes(&frames, &acquired)
        } else {
            Acquisition::Static(&style, &acquired)
        };
        // When
        let result = application::producer(acquisition, &Theme::default(), &shared());
        // Then
        assert_eq!(result, Err(KernelError::Authority));
    }

    #[test]
    #[serial_test::serial]
    fn application_checks_member_policy_independently_of_real_parent_receipt() {
        // Given
        let _state = state();
        let mut frames = fixture("a", ExtractKeyframes::default);
        let acquired = frames.counter_produce(None).required("receipt");
        frames.keyframes.insert(
            "from".into(),
            vec![ExtractStaticStyle::new("color", "red", 0, None)],
        );
        // When
        let result = application::apply(
            Acquisition::Keyframes(&frames, &acquired),
            &Theme::default(),
            &shared(),
        );
        // Then
        assert_eq!(
            result,
            Err(KernelError::Producer(CounterProducerError::WrongPolicy))
        );
    }
}
