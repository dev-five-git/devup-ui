use css::allocation_input::{
    AllocationContext, LegacyDeclaration, LegacyInput, LegacyVariable, NameMode, capture_context,
};
use css::counter_names::{AllocatedName, allocate_name};
use css::optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value};
use css::{CounterOwner, Site, sparse_site::SourceFile};

use super::counter_producer_selector::class_selector;
use super::{
    ExtractDynamicStyle, ExtractKeyframes, ProducerPolicy, extract_static_style::ExtractStaticStyle,
};

/// Actual allocation receipt; original authority remains present even for shared names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducedAllocation {
    pub original: u32,
    pub input: LegacyInput,
    pub context: AllocationContext,
    pub allocation: AllocatedName,
}

/// Class-to-variable association, with separate no-site variable allocation evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProducedDynamic {
    pub class: ProducedAllocation,
    pub variable: String,
    pub variable_allocation: Option<ProducedAllocation>,
    pub site: Option<Site>,
    pub identifier: String,
    pub important: bool,
}

/// Invalid dormant producer authority, checked before any counter reservation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterProducerError {
    /// The parent or a keyframe member was constructed under Current policy.
    WrongPolicy,
    /// The assignment site has no numeric original authority.
    UnnumberedSite,
}

impl std::fmt::Display for CounterProducerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongPolicy => {
                f.write_str("counter producer requires retained CounterOriginal policy")
            }
            Self::UnnumberedSite => {
                f.write_str("counter producer requires a numbered assignment site")
            }
        }
    }
}

impl std::error::Error for CounterProducerError {}

const fn retained_original(policy: ProducerPolicy) -> Result<u32, CounterProducerError> {
    match policy {
        ProducerPolicy::Current => Err(CounterProducerError::WrongPolicy),
        ProducerPolicy::CounterOriginal(original) => Ok(original),
    }
}

fn produce(original: u32, input: LegacyInput, context: AllocationContext) -> ProducedAllocation {
    let allocation = allocate_name(&input, &context);
    ProducedAllocation {
        original,
        input,
        context,
        allocation,
    }
}

pub(super) fn produce_static(
    style: &ExtractStaticStyle,
    filename: Option<&str>,
) -> Result<ProducedAllocation, CounterProducerError> {
    let original = retained_original(style.producer_policy())?;
    let value = style.resolved_value();
    let value = if style.property() != "content" && check_multi_css_optimize(style.property()) {
        optimize_multi_css_value(&value).into_owned()
    } else {
        value.into_owned()
    };
    let mut declaration = LegacyDeclaration {
        property: style.property().to_string(),
        level: style.level(),
        value: Some(value),
        selector: None,
        order: style.style_order(),
    };
    let context = capture_context(
        &LegacyInput::Declaration(declaration.clone()),
        filename,
        CounterOwner::D9(original),
    );
    declaration.selector = class_selector(style.selector(), style.layer(), context.config.mode);
    Ok(produce(
        original,
        LegacyInput::Declaration(declaration),
        context,
    ))
}

pub(super) fn produce_keyframes(
    frames: &ExtractKeyframes,
    filename: Option<&str>,
) -> Result<ProducedAllocation, CounterProducerError> {
    let original = retained_original(frames.producer_policy())?;
    for style in frames.keyframes.values().flatten() {
        retained_original(style.producer_policy())?;
    }
    let context = capture_context(
        &LegacyInput::Keyframes(String::new()),
        filename,
        CounterOwner::D9(original),
    );
    let input = LegacyInput::Keyframes(super::counter_keyframe_input::input(
        frames,
        context.config.mode,
    ));
    Ok(produce(original, input, context))
}

pub(super) fn produce_dynamic(
    style: &ExtractDynamicStyle,
    filename: Option<&str>,
) -> Result<ProducedDynamic, CounterProducerError> {
    let original = retained_original(style.producer_policy())?;
    if let Some(site) = style.site() {
        match &site.file {
            SourceFile::D9(_) => {}
            SourceFile::Unnumbered(_) => return Err(CounterProducerError::UnnumberedSite),
        }
    }
    let mut declaration = LegacyDeclaration {
        property: style.property().to_string(),
        level: style.level(),
        value: style.important().then(|| "!important".to_string()),
        selector: None,
        order: style.style_order(),
    };
    let context = capture_context(
        &LegacyInput::Declaration(declaration.clone()),
        filename,
        CounterOwner::D9(original),
    );
    declaration.selector = class_selector(style.selector(), style.layer(), context.config.mode);
    let variable_input = LegacyInput::Variable(LegacyVariable {
        property: style.property().to_string(),
        level: style.level(),
        selector: declaration.selector.clone(),
    });
    let variable_context = AllocationContext {
        config: context.config.clone(),
        file: None,
        delivery: None,
    };
    let (class, variable, variable_allocation) = match style.site() {
        Some(site) => {
            let variable = site.variable_name(&context.config.prefix);
            declaration.value = Some(format!(
                "var({variable}){}",
                if style.important() { " !important" } else { "" }
            ));
            (
                produce(original, LegacyInput::Declaration(declaration), context),
                variable,
                None,
            )
        }
        None => match context.config.mode {
            NameMode::Counter | NameMode::Debug => {
                let class = produce(original, LegacyInput::Declaration(declaration), context);
                let allocated = produce(original, variable_input, variable_context);
                (class, allocated.allocation.name.clone(), Some(allocated))
            }
            NameMode::AtomHoist => {
                let allocated = produce(original, variable_input, variable_context);
                let variable = allocated.allocation.name.clone();
                declaration.value = Some(format!(
                    "var({variable}){}",
                    if style.important() { " !important" } else { "" }
                ));
                (
                    produce(original, LegacyInput::Declaration(declaration), context),
                    variable,
                    Some(allocated),
                )
            }
        },
    };
    Ok(ProducedDynamic {
        class,
        variable,
        variable_allocation,
        site: style.site().cloned(),
        identifier: style.identifier().to_string(),
        important: style.important(),
    })
}
