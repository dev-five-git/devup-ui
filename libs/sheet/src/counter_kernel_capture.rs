use super::error::KernelError;
use crate::{
    emission_seed::{DeclarationSeed, FrozenPreset, FrozenTypography, NumericSite, preset_key},
    theme::Theme,
};
use extractor::extract_style::{
    CounterProducerError, ExtractDynamicStyle,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
};

pub(super) fn numeric_site(site: Option<&css::Site>) -> Result<Option<NumericSite>, KernelError> {
    site.map(|site| match &site.file {
        css::sparse_site::SourceFile::D9(source) => Ok(NumericSite {
            source: *source,
            at: site.at,
            role: site.role,
        }),
        css::sparse_site::SourceFile::Unnumbered(_) => {
            Err(KernelError::Producer(CounterProducerError::UnnumberedSite))
        }
    })
    .transpose()
}

pub(super) fn declaration(
    style: &ExtractStaticStyle,
    theme: &Theme,
) -> Result<DeclarationSeed, KernelError> {
    let first = match style.theme_token_resolution {
        ThemeTokenResolution::CssVariable => None,
        ThemeTokenResolution::FirstValue => {
            css::theme_tokens::get_first_theme_token_value(&style.property, &style.value)
        }
    };
    let preset = if style.property == "typography" {
        let (name, yielded) = preset_key(&style.value);
        let preset = FrozenPreset {
            name: name.into(),
            frames: theme.typography.get(name).map_or_else(Vec::new, |frames| {
                frames
                    .0
                    .iter()
                    .map(|frame| frame.as_ref().map(FrozenTypography::from))
                    .collect()
            }),
        };
        if preset.declarations(style.level, &yielded)
            != css::content_typography::declarations(&style.value, style.level)
        {
            return Err(KernelError::Preset);
        }
        Some(preset)
    } else {
        None
    };
    Ok(DeclarationSeed::capture(style, first, preset))
}

pub(super) fn dynamic(style: &ExtractDynamicStyle) -> DeclarationSeed {
    DeclarationSeed {
        property: style.property().into(),
        value: String::new(),
        level: style.level(),
        selector: style.selector().cloned(),
        style_order: style.style_order(),
        layer: style.layer().map(str::to_string),
        resolution: crate::emission_seed::Resolution::CssVariable,
        first_value: None,
        preset: None,
    }
}
