use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use css::{
    allocation_input::{LegacyDeclaration, LegacyInput, LegacyVariable, NameMode},
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value},
};
use extractor::extract_style::extract_static_style::ThemeTokenResolution;

use crate::emission_seed::{DeclarationSeed, EmissionInput, Resolution};

/// Legacy keyframe hashing deliberately excludes newer producer metadata.
struct LegacyMember<'a>(&'a DeclarationSeed);

impl Hash for LegacyMember<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let member = self.0;
        member.property.hash(state);
        member.value.hash(state);
        member.level.hash(state);
        member.selector.hash(state);
        member.style_order.hash(state);
        member.layer.hash(state);
        match member.resolution {
            Resolution::CssVariable => ThemeTokenResolution::CssVariable,
            Resolution::FirstValue => ThemeTokenResolution::FirstValue,
        }
        .hash(state);
    }
}

pub(super) fn selector(declaration: &DeclarationSeed, mode: NameMode) -> Option<String> {
    match mode {
        NameMode::AtomHoist => Some(css::atom_name::selector_key(
            declaration.selector.as_ref(),
            declaration.layer.as_deref(),
        )),
        NameMode::Counter | NameMode::Debug => {
            let selector = declaration
                .selector
                .as_ref()
                .map(|selector| selector.as_class_str().into_owned());
            match &declaration.layer {
                Some(layer) => Some(format!(
                    "{}@layer {layer}",
                    selector.as_deref().unwrap_or_default()
                )),
                None => selector,
            }
        }
    }
}

pub(super) fn variable(declaration: &DeclarationSeed, mode: NameMode) -> LegacyInput {
    LegacyInput::Variable(LegacyVariable {
        property: declaration.property.clone(),
        level: declaration.level,
        selector: selector(declaration, mode),
    })
}

pub(super) fn input(body: &EmissionInput, mode: NameMode) -> LegacyInput {
    let (declaration, value) = match body {
        EmissionInput::Static(declaration) | EmissionInput::Typography(declaration) => {
            let value = if declaration.property == "typography" {
                &declaration.value
            } else {
                match declaration.resolution {
                    Resolution::CssVariable => &declaration.value,
                    Resolution::FirstValue => declaration
                        .first_value
                        .as_ref()
                        .unwrap_or(&declaration.value),
                }
            };
            let value = if declaration.property != "content"
                && check_multi_css_optimize(&declaration.property)
            {
                optimize_multi_css_value(value).into_owned()
            } else {
                value.clone()
            };
            (declaration, Some(value))
        }
        EmissionInput::Dynamic {
            declaration,
            variable,
            site,
            important,
        } => {
            let value = match (site, mode) {
                (None, NameMode::Counter | NameMode::Debug) => {
                    important.then(|| "!important".into())
                }
                (Some(_), NameMode::Counter | NameMode::Debug | NameMode::AtomHoist)
                | (None, NameMode::AtomHoist) => Some(format!(
                    "var({variable}){}",
                    if *important { " !important" } else { "" }
                )),
            };
            (declaration, value)
        }
        EmissionInput::Keyframes { steps } => {
            return LegacyInput::Keyframes(keyframes(steps, mode));
        }
    };
    LegacyInput::Declaration(LegacyDeclaration {
        property: declaration.property.clone(),
        level: declaration.level,
        value,
        selector: selector(declaration, mode),
        order: declaration.style_order,
    })
}

fn keyframes(steps: &BTreeMap<String, Vec<DeclarationSeed>>, mode: NameMode) -> String {
    match mode {
        NameMode::Counter | NameMode::Debug => {
            let mut hasher = DefaultHasher::new();
            steps
                .iter()
                .map(|(step, members)| (step, members.iter().map(LegacyMember).collect()))
                .collect::<BTreeMap<_, Vec<_>>>()
                .hash(&mut hasher);
            hasher.finish().to_string()
        }
        NameMode::AtomHoist => {
            let mut result = String::new();
            for (step, members) in steps {
                result.push_str(&css::atom_name::hex(step));
                result.push('{');
                for member in members {
                    result.push_str(&css::atom_name::hex(&member.property));
                    result.push(':');
                    result.push_str(&css::atom_name::hex(&member.value));
                    result.push(';');
                }
                result.push('}');
            }
            result
        }
    }
}
