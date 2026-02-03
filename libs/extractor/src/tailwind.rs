//! Tailwind CSS class parser for devup-ui extraction
//!
//! This module parses Tailwind CSS class strings and converts them to
//! ExtractStyleValue objects for integration with the devup-ui extraction system.

// The nested if-let pattern is intentional for readability in parsing code.
// Using if-let chains would make the code harder to read and modify.
#![allow(clippy::collapsible_if)]

use css::style_selector::StyleSelector;
use phf::phf_map;

use crate::extract_style::{
    extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
};

/// Responsive breakpoint levels matching devup-ui convention
/// 0 = base (no prefix)
/// 1 = sm (640px)
/// 2 = md (768px)
/// 3 = lg (1024px)
/// 4 = xl (1280px)
/// 5 = 2xl (1536px)
///
/// Map of responsive prefix to level
static RESPONSIVE_PREFIX_MAP: phf::Map<&'static str, u8> = phf_map! {
    "sm" => 1,
    "md" => 2,
    "lg" => 3,
    "xl" => 4,
    "2xl" => 5,
};

/// Variant prefixes that map to CSS pseudo-classes/selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TailwindVariant {
    Hover,
    Focus,
    FocusVisible,
    FocusWithin,
    Active,
    Visited,
    Disabled,
    Enabled,
    Checked,
    Indeterminate,
    Default,
    Required,
    Valid,
    Invalid,
    InRange,
    OutOfRange,
    PlaceholderShown,
    Autofill,
    ReadOnly,
    FirstChild,
    LastChild,
    OnlyChild,
    OddChild,
    EvenChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    Empty,
    Target,
    Open,
    Dark,
    Placeholder,
    Before,
    After,
    Selection,
    Marker,
    FirstLetter,
    FirstLine,
    Backdrop,
    File,
    GroupHover,
    GroupFocus,
    GroupActive,
    GroupDisabled,
    PeerHover,
    PeerFocus,
    PeerActive,
    PeerDisabled,
    PeerChecked,
    PeerInvalid,
    Print,
    Screen,
    Portrait,
    Landscape,
    MotionReduce,
    MotionSafe,
    ContrastMore,
    ContrastLess,
    ForcedColors,
    Rtl,
    Ltr,
}

impl TailwindVariant {
    /// Convert variant to StyleSelector
    pub fn to_selector(self) -> StyleSelector {
        match self {
            TailwindVariant::Hover => StyleSelector::Selector("&:hover".to_string()),
            TailwindVariant::Focus => StyleSelector::Selector("&:focus".to_string()),
            TailwindVariant::FocusVisible => StyleSelector::Selector("&:focus-visible".to_string()),
            TailwindVariant::FocusWithin => StyleSelector::Selector("&:focus-within".to_string()),
            TailwindVariant::Active => StyleSelector::Selector("&:active".to_string()),
            TailwindVariant::Visited => StyleSelector::Selector("&:visited".to_string()),
            TailwindVariant::Disabled => StyleSelector::Selector("&:disabled".to_string()),
            TailwindVariant::Enabled => StyleSelector::Selector("&:enabled".to_string()),
            TailwindVariant::Checked => StyleSelector::Selector("&:checked".to_string()),
            TailwindVariant::Indeterminate => {
                StyleSelector::Selector("&:indeterminate".to_string())
            }
            TailwindVariant::Default => StyleSelector::Selector("&:default".to_string()),
            TailwindVariant::Required => StyleSelector::Selector("&:required".to_string()),
            TailwindVariant::Valid => StyleSelector::Selector("&:valid".to_string()),
            TailwindVariant::Invalid => StyleSelector::Selector("&:invalid".to_string()),
            TailwindVariant::InRange => StyleSelector::Selector("&:in-range".to_string()),
            TailwindVariant::OutOfRange => StyleSelector::Selector("&:out-of-range".to_string()),
            TailwindVariant::PlaceholderShown => {
                StyleSelector::Selector("&:placeholder-shown".to_string())
            }
            TailwindVariant::Autofill => StyleSelector::Selector("&:autofill".to_string()),
            TailwindVariant::ReadOnly => StyleSelector::Selector("&:read-only".to_string()),
            TailwindVariant::FirstChild => StyleSelector::Selector("&:first-child".to_string()),
            TailwindVariant::LastChild => StyleSelector::Selector("&:last-child".to_string()),
            TailwindVariant::OnlyChild => StyleSelector::Selector("&:only-child".to_string()),
            TailwindVariant::OddChild => StyleSelector::Selector("&:nth-child(odd)".to_string()),
            TailwindVariant::EvenChild => StyleSelector::Selector("&:nth-child(even)".to_string()),
            TailwindVariant::FirstOfType => StyleSelector::Selector("&:first-of-type".to_string()),
            TailwindVariant::LastOfType => StyleSelector::Selector("&:last-of-type".to_string()),
            TailwindVariant::OnlyOfType => StyleSelector::Selector("&:only-of-type".to_string()),
            TailwindVariant::Empty => StyleSelector::Selector("&:empty".to_string()),
            TailwindVariant::Target => StyleSelector::Selector("&:target".to_string()),
            TailwindVariant::Open => StyleSelector::Selector("&[open]".to_string()),
            TailwindVariant::Dark => {
                StyleSelector::Selector(":root[data-theme=dark] &".to_string())
            }
            TailwindVariant::Placeholder => StyleSelector::Selector("&::placeholder".to_string()),
            TailwindVariant::Before => StyleSelector::Selector("&::before".to_string()),
            TailwindVariant::After => StyleSelector::Selector("&::after".to_string()),
            TailwindVariant::Selection => StyleSelector::Selector("&::selection".to_string()),
            TailwindVariant::Marker => StyleSelector::Selector("&::marker".to_string()),
            TailwindVariant::FirstLetter => StyleSelector::Selector("&::first-letter".to_string()),
            TailwindVariant::FirstLine => StyleSelector::Selector("&::first-line".to_string()),
            TailwindVariant::Backdrop => StyleSelector::Selector("&::backdrop".to_string()),
            TailwindVariant::File => StyleSelector::Selector("&::file-selector-button".to_string()),
            TailwindVariant::GroupHover => {
                StyleSelector::Selector("*[role=group]:hover &".to_string())
            }
            TailwindVariant::GroupFocus => {
                StyleSelector::Selector("*[role=group]:focus &".to_string())
            }
            TailwindVariant::GroupActive => {
                StyleSelector::Selector("*[role=group]:active &".to_string())
            }
            TailwindVariant::GroupDisabled => {
                StyleSelector::Selector("*[role=group]:disabled &".to_string())
            }
            TailwindVariant::PeerHover => StyleSelector::Selector(".peer:hover ~ &".to_string()),
            TailwindVariant::PeerFocus => StyleSelector::Selector(".peer:focus ~ &".to_string()),
            TailwindVariant::PeerActive => StyleSelector::Selector(".peer:active ~ &".to_string()),
            TailwindVariant::PeerDisabled => {
                StyleSelector::Selector(".peer:disabled ~ &".to_string())
            }
            TailwindVariant::PeerChecked => {
                StyleSelector::Selector(".peer:checked ~ &".to_string())
            }
            TailwindVariant::PeerInvalid => {
                StyleSelector::Selector(".peer:invalid ~ &".to_string())
            }
            TailwindVariant::Print => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "print".to_string(),
                selector: None,
            },
            TailwindVariant::Screen => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "screen".to_string(),
                selector: None,
            },
            TailwindVariant::Portrait => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(orientation: portrait)".to_string(),
                selector: None,
            },
            TailwindVariant::Landscape => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(orientation: landscape)".to_string(),
                selector: None,
            },
            TailwindVariant::MotionReduce => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(prefers-reduced-motion: reduce)".to_string(),
                selector: None,
            },
            TailwindVariant::MotionSafe => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(prefers-reduced-motion: no-preference)".to_string(),
                selector: None,
            },
            TailwindVariant::ContrastMore => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(prefers-contrast: more)".to_string(),
                selector: None,
            },
            TailwindVariant::ContrastLess => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(prefers-contrast: less)".to_string(),
                selector: None,
            },
            TailwindVariant::ForcedColors => StyleSelector::At {
                kind: css::style_selector::AtRuleKind::Media,
                query: "(forced-colors: active)".to_string(),
                selector: None,
            },
            TailwindVariant::Rtl => StyleSelector::Selector("[dir=rtl] &".to_string()),
            TailwindVariant::Ltr => StyleSelector::Selector("[dir=ltr] &".to_string()),
        }
    }

    /// Parse variant from string prefix
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "hover" => Some(TailwindVariant::Hover),
            "focus" => Some(TailwindVariant::Focus),
            "focus-visible" => Some(TailwindVariant::FocusVisible),
            "focus-within" => Some(TailwindVariant::FocusWithin),
            "active" => Some(TailwindVariant::Active),
            "visited" => Some(TailwindVariant::Visited),
            "disabled" => Some(TailwindVariant::Disabled),
            "enabled" => Some(TailwindVariant::Enabled),
            "checked" => Some(TailwindVariant::Checked),
            "indeterminate" => Some(TailwindVariant::Indeterminate),
            "default" => Some(TailwindVariant::Default),
            "required" => Some(TailwindVariant::Required),
            "valid" => Some(TailwindVariant::Valid),
            "invalid" => Some(TailwindVariant::Invalid),
            "in-range" => Some(TailwindVariant::InRange),
            "out-of-range" => Some(TailwindVariant::OutOfRange),
            "placeholder-shown" => Some(TailwindVariant::PlaceholderShown),
            "autofill" => Some(TailwindVariant::Autofill),
            "read-only" => Some(TailwindVariant::ReadOnly),
            "first" => Some(TailwindVariant::FirstChild),
            "last" => Some(TailwindVariant::LastChild),
            "only" => Some(TailwindVariant::OnlyChild),
            "odd" => Some(TailwindVariant::OddChild),
            "even" => Some(TailwindVariant::EvenChild),
            "first-of-type" => Some(TailwindVariant::FirstOfType),
            "last-of-type" => Some(TailwindVariant::LastOfType),
            "only-of-type" => Some(TailwindVariant::OnlyOfType),
            "empty" => Some(TailwindVariant::Empty),
            "target" => Some(TailwindVariant::Target),
            "open" => Some(TailwindVariant::Open),
            "dark" => Some(TailwindVariant::Dark),
            "placeholder" => Some(TailwindVariant::Placeholder),
            "before" => Some(TailwindVariant::Before),
            "after" => Some(TailwindVariant::After),
            "selection" => Some(TailwindVariant::Selection),
            "marker" => Some(TailwindVariant::Marker),
            "first-letter" => Some(TailwindVariant::FirstLetter),
            "first-line" => Some(TailwindVariant::FirstLine),
            "backdrop" => Some(TailwindVariant::Backdrop),
            "file" => Some(TailwindVariant::File),
            "group-hover" => Some(TailwindVariant::GroupHover),
            "group-focus" => Some(TailwindVariant::GroupFocus),
            "group-active" => Some(TailwindVariant::GroupActive),
            "group-disabled" => Some(TailwindVariant::GroupDisabled),
            "peer-hover" => Some(TailwindVariant::PeerHover),
            "peer-focus" => Some(TailwindVariant::PeerFocus),
            "peer-active" => Some(TailwindVariant::PeerActive),
            "peer-disabled" => Some(TailwindVariant::PeerDisabled),
            "peer-checked" => Some(TailwindVariant::PeerChecked),
            "peer-invalid" => Some(TailwindVariant::PeerInvalid),
            "print" => Some(TailwindVariant::Print),
            "screen" => Some(TailwindVariant::Screen),
            "portrait" => Some(TailwindVariant::Portrait),
            "landscape" => Some(TailwindVariant::Landscape),
            "motion-reduce" => Some(TailwindVariant::MotionReduce),
            "motion-safe" => Some(TailwindVariant::MotionSafe),
            "contrast-more" => Some(TailwindVariant::ContrastMore),
            "contrast-less" => Some(TailwindVariant::ContrastLess),
            "forced-colors" => Some(TailwindVariant::ForcedColors),
            "rtl" => Some(TailwindVariant::Rtl),
            "ltr" => Some(TailwindVariant::Ltr),
            _ => None,
        }
    }
}

/// Parsed Tailwind class with all components
#[derive(Debug, Clone, PartialEq)]
pub struct TailwindClass {
    /// Responsive level (0=base, 1=sm, 2=md, 3=lg, 4=xl, 5=2xl)
    pub responsive: u8,
    /// Variants/modifiers applied
    pub variants: Vec<TailwindVariant>,
    /// CSS property name
    pub property: String,
    /// CSS value
    pub value: String,
    /// Whether this is a negative value
    pub negative: bool,
}

impl TailwindClass {
    /// Convert to ExtractStaticStyle
    pub fn to_static_style(&self) -> ExtractStaticStyle {
        // For transform property, negative is already incorporated into the value
        // (e.g., translateX(-1rem)), so don't add prefix again
        let value = if self.negative && self.property != "transform" {
            format!("-{}", self.value)
        } else {
            self.value.clone()
        };

        let selector = if self.variants.is_empty() {
            None
        } else {
            // Combine multiple variants into a single selector
            Some(self.combine_selectors())
        };

        ExtractStaticStyle::new(&self.property, &value, self.responsive, selector)
    }

    /// Combine multiple variant selectors
    fn combine_selectors(&self) -> StyleSelector {
        if self.variants.len() == 1 {
            return self.variants[0].to_selector();
        }

        // For multiple variants, combine them
        // e.g., dark:hover: becomes :root[data-theme=dark] &:hover
        let mut selector_str = String::new();
        let mut has_at_rule = None;

        for variant in &self.variants {
            let sel = variant.to_selector();
            match sel {
                StyleSelector::Selector(s) => {
                    if selector_str.is_empty() {
                        selector_str = s;
                    } else {
                        // Combine selectors
                        selector_str =
                            format!("{}{}", selector_str.replace(" &", ""), s.replace("&", ""));
                        if !selector_str.contains(" &") && !selector_str.ends_with(" &") {
                            selector_str.push_str(" &");
                        }
                    }
                }
                StyleSelector::At { kind, query, .. } => {
                    has_at_rule = Some((kind, query));
                }
                StyleSelector::Global(_, _) => {}
            }
        }

        if let Some((kind, query)) = has_at_rule {
            StyleSelector::At {
                kind,
                query,
                selector: if selector_str.is_empty() {
                    None
                } else {
                    Some(selector_str)
                },
            }
        } else {
            StyleSelector::Selector(selector_str)
        }
    }
}

/// Tailwind color values
static TAILWIND_COLORS: phf::Map<&'static str, &'static str> = phf_map! {
    // Inherit/Current/Transparent
    "inherit" => "inherit",
    "current" => "currentColor",
    "transparent" => "transparent",
    // Black and White
    "black" => "#000",
    "white" => "#fff",
    // Slate
    "slate-50" => "#f8fafc",
    "slate-100" => "#f1f5f9",
    "slate-200" => "#e2e8f0",
    "slate-300" => "#cbd5e1",
    "slate-400" => "#94a3b8",
    "slate-500" => "#64748b",
    "slate-600" => "#475569",
    "slate-700" => "#334155",
    "slate-800" => "#1e293b",
    "slate-900" => "#0f172a",
    "slate-950" => "#020617",
    // Gray
    "gray-50" => "#f9fafb",
    "gray-100" => "#f3f4f6",
    "gray-200" => "#e5e7eb",
    "gray-300" => "#d1d5db",
    "gray-400" => "#9ca3af",
    "gray-500" => "#6b7280",
    "gray-600" => "#4b5563",
    "gray-700" => "#374151",
    "gray-800" => "#1f2937",
    "gray-900" => "#111827",
    "gray-950" => "#030712",
    // Zinc
    "zinc-50" => "#fafafa",
    "zinc-100" => "#f4f4f5",
    "zinc-200" => "#e4e4e7",
    "zinc-300" => "#d4d4d8",
    "zinc-400" => "#a1a1aa",
    "zinc-500" => "#71717a",
    "zinc-600" => "#52525b",
    "zinc-700" => "#3f3f46",
    "zinc-800" => "#27272a",
    "zinc-900" => "#18181b",
    "zinc-950" => "#09090b",
    // Neutral
    "neutral-50" => "#fafafa",
    "neutral-100" => "#f5f5f5",
    "neutral-200" => "#e5e5e5",
    "neutral-300" => "#d4d4d4",
    "neutral-400" => "#a3a3a3",
    "neutral-500" => "#737373",
    "neutral-600" => "#525252",
    "neutral-700" => "#404040",
    "neutral-800" => "#262626",
    "neutral-900" => "#171717",
    "neutral-950" => "#0a0a0a",
    // Stone
    "stone-50" => "#fafaf9",
    "stone-100" => "#f5f5f4",
    "stone-200" => "#e7e5e4",
    "stone-300" => "#d6d3d1",
    "stone-400" => "#a8a29e",
    "stone-500" => "#78716c",
    "stone-600" => "#57534e",
    "stone-700" => "#44403c",
    "stone-800" => "#292524",
    "stone-900" => "#1c1917",
    "stone-950" => "#0c0a09",
    // Red
    "red-50" => "#fef2f2",
    "red-100" => "#fee2e2",
    "red-200" => "#fecaca",
    "red-300" => "#fca5a5",
    "red-400" => "#f87171",
    "red-500" => "#ef4444",
    "red-600" => "#dc2626",
    "red-700" => "#b91c1c",
    "red-800" => "#991b1b",
    "red-900" => "#7f1d1d",
    "red-950" => "#450a0a",
    // Orange
    "orange-50" => "#fff7ed",
    "orange-100" => "#ffedd5",
    "orange-200" => "#fed7aa",
    "orange-300" => "#fdba74",
    "orange-400" => "#fb923c",
    "orange-500" => "#f97316",
    "orange-600" => "#ea580c",
    "orange-700" => "#c2410c",
    "orange-800" => "#9a3412",
    "orange-900" => "#7c2d12",
    "orange-950" => "#431407",
    // Amber
    "amber-50" => "#fffbeb",
    "amber-100" => "#fef3c7",
    "amber-200" => "#fde68a",
    "amber-300" => "#fcd34d",
    "amber-400" => "#fbbf24",
    "amber-500" => "#f59e0b",
    "amber-600" => "#d97706",
    "amber-700" => "#b45309",
    "amber-800" => "#92400e",
    "amber-900" => "#78350f",
    "amber-950" => "#451a03",
    // Yellow
    "yellow-50" => "#fefce8",
    "yellow-100" => "#fef9c3",
    "yellow-200" => "#fef08a",
    "yellow-300" => "#fde047",
    "yellow-400" => "#facc15",
    "yellow-500" => "#eab308",
    "yellow-600" => "#ca8a04",
    "yellow-700" => "#a16207",
    "yellow-800" => "#854d0e",
    "yellow-900" => "#713f12",
    "yellow-950" => "#422006",
    // Lime
    "lime-50" => "#f7fee7",
    "lime-100" => "#ecfccb",
    "lime-200" => "#d9f99d",
    "lime-300" => "#bef264",
    "lime-400" => "#a3e635",
    "lime-500" => "#84cc16",
    "lime-600" => "#65a30d",
    "lime-700" => "#4d7c0f",
    "lime-800" => "#3f6212",
    "lime-900" => "#365314",
    "lime-950" => "#1a2e05",
    // Green
    "green-50" => "#f0fdf4",
    "green-100" => "#dcfce7",
    "green-200" => "#bbf7d0",
    "green-300" => "#86efac",
    "green-400" => "#4ade80",
    "green-500" => "#22c55e",
    "green-600" => "#16a34a",
    "green-700" => "#15803d",
    "green-800" => "#166534",
    "green-900" => "#14532d",
    "green-950" => "#052e16",
    // Emerald
    "emerald-50" => "#ecfdf5",
    "emerald-100" => "#d1fae5",
    "emerald-200" => "#a7f3d0",
    "emerald-300" => "#6ee7b7",
    "emerald-400" => "#34d399",
    "emerald-500" => "#10b981",
    "emerald-600" => "#059669",
    "emerald-700" => "#047857",
    "emerald-800" => "#065f46",
    "emerald-900" => "#064e3b",
    "emerald-950" => "#022c22",
    // Teal
    "teal-50" => "#f0fdfa",
    "teal-100" => "#ccfbf1",
    "teal-200" => "#99f6e4",
    "teal-300" => "#5eead4",
    "teal-400" => "#2dd4bf",
    "teal-500" => "#14b8a6",
    "teal-600" => "#0d9488",
    "teal-700" => "#0f766e",
    "teal-800" => "#115e59",
    "teal-900" => "#134e4a",
    "teal-950" => "#042f2e",
    // Cyan
    "cyan-50" => "#ecfeff",
    "cyan-100" => "#cffafe",
    "cyan-200" => "#a5f3fc",
    "cyan-300" => "#67e8f9",
    "cyan-400" => "#22d3ee",
    "cyan-500" => "#06b6d4",
    "cyan-600" => "#0891b2",
    "cyan-700" => "#0e7490",
    "cyan-800" => "#155e75",
    "cyan-900" => "#164e63",
    "cyan-950" => "#083344",
    // Sky
    "sky-50" => "#f0f9ff",
    "sky-100" => "#e0f2fe",
    "sky-200" => "#bae6fd",
    "sky-300" => "#7dd3fc",
    "sky-400" => "#38bdf8",
    "sky-500" => "#0ea5e9",
    "sky-600" => "#0284c7",
    "sky-700" => "#0369a1",
    "sky-800" => "#075985",
    "sky-900" => "#0c4a6e",
    "sky-950" => "#082f49",
    // Blue
    "blue-50" => "#eff6ff",
    "blue-100" => "#dbeafe",
    "blue-200" => "#bfdbfe",
    "blue-300" => "#93c5fd",
    "blue-400" => "#60a5fa",
    "blue-500" => "#3b82f6",
    "blue-600" => "#2563eb",
    "blue-700" => "#1d4ed8",
    "blue-800" => "#1e40af",
    "blue-900" => "#1e3a8a",
    "blue-950" => "#172554",
    // Indigo
    "indigo-50" => "#eef2ff",
    "indigo-100" => "#e0e7ff",
    "indigo-200" => "#c7d2fe",
    "indigo-300" => "#a5b4fc",
    "indigo-400" => "#818cf8",
    "indigo-500" => "#6366f1",
    "indigo-600" => "#4f46e5",
    "indigo-700" => "#4338ca",
    "indigo-800" => "#3730a3",
    "indigo-900" => "#312e81",
    "indigo-950" => "#1e1b4b",
    // Violet
    "violet-50" => "#f5f3ff",
    "violet-100" => "#ede9fe",
    "violet-200" => "#ddd6fe",
    "violet-300" => "#c4b5fd",
    "violet-400" => "#a78bfa",
    "violet-500" => "#8b5cf6",
    "violet-600" => "#7c3aed",
    "violet-700" => "#6d28d9",
    "violet-800" => "#5b21b6",
    "violet-900" => "#4c1d95",
    "violet-950" => "#2e1065",
    // Purple
    "purple-50" => "#faf5ff",
    "purple-100" => "#f3e8ff",
    "purple-200" => "#e9d5ff",
    "purple-300" => "#d8b4fe",
    "purple-400" => "#c084fc",
    "purple-500" => "#a855f7",
    "purple-600" => "#9333ea",
    "purple-700" => "#7e22ce",
    "purple-800" => "#6b21a8",
    "purple-900" => "#581c87",
    "purple-950" => "#3b0764",
    // Fuchsia
    "fuchsia-50" => "#fdf4ff",
    "fuchsia-100" => "#fae8ff",
    "fuchsia-200" => "#f5d0fe",
    "fuchsia-300" => "#f0abfc",
    "fuchsia-400" => "#e879f9",
    "fuchsia-500" => "#d946ef",
    "fuchsia-600" => "#c026d3",
    "fuchsia-700" => "#a21caf",
    "fuchsia-800" => "#86198f",
    "fuchsia-900" => "#701a75",
    "fuchsia-950" => "#4a044e",
    // Pink
    "pink-50" => "#fdf2f8",
    "pink-100" => "#fce7f3",
    "pink-200" => "#fbcfe8",
    "pink-300" => "#f9a8d4",
    "pink-400" => "#f472b6",
    "pink-500" => "#ec4899",
    "pink-600" => "#db2777",
    "pink-700" => "#be185d",
    "pink-800" => "#9d174d",
    "pink-900" => "#831843",
    "pink-950" => "#500724",
    // Rose
    "rose-50" => "#fff1f2",
    "rose-100" => "#ffe4e6",
    "rose-200" => "#fecdd3",
    "rose-300" => "#fda4af",
    "rose-400" => "#fb7185",
    "rose-500" => "#f43f5e",
    "rose-600" => "#e11d48",
    "rose-700" => "#be123c",
    "rose-800" => "#9f1239",
    "rose-900" => "#881337",
    "rose-950" => "#4c0519",
};

/// Spacing scale (Tailwind default: 1 unit = 0.25rem = 4px)
static SPACING_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0px",
    "px" => "1px",
    "0.5" => "0.125rem",
    "1" => "0.25rem",
    "1.5" => "0.375rem",
    "2" => "0.5rem",
    "2.5" => "0.625rem",
    "3" => "0.75rem",
    "3.5" => "0.875rem",
    "4" => "1rem",
    "5" => "1.25rem",
    "6" => "1.5rem",
    "7" => "1.75rem",
    "8" => "2rem",
    "9" => "2.25rem",
    "10" => "2.5rem",
    "11" => "2.75rem",
    "12" => "3rem",
    "14" => "3.5rem",
    "16" => "4rem",
    "20" => "5rem",
    "24" => "6rem",
    "28" => "7rem",
    "32" => "8rem",
    "36" => "9rem",
    "40" => "10rem",
    "44" => "11rem",
    "48" => "12rem",
    "52" => "13rem",
    "56" => "14rem",
    "60" => "15rem",
    "64" => "16rem",
    "72" => "18rem",
    "80" => "20rem",
    "96" => "24rem",
    "auto" => "auto",
    "full" => "100%",
    "1/2" => "50%",
    "1/3" => "33.333333%",
    "2/3" => "66.666667%",
    "1/4" => "25%",
    "2/4" => "50%",
    "3/4" => "75%",
    "1/5" => "20%",
    "2/5" => "40%",
    "3/5" => "60%",
    "4/5" => "80%",
    "1/6" => "16.666667%",
    "2/6" => "33.333333%",
    "3/6" => "50%",
    "4/6" => "66.666667%",
    "5/6" => "83.333333%",
    "1/12" => "8.333333%",
    "2/12" => "16.666667%",
    "3/12" => "25%",
    "4/12" => "33.333333%",
    "5/12" => "41.666667%",
    "6/12" => "50%",
    "7/12" => "58.333333%",
    "8/12" => "66.666667%",
    "9/12" => "75%",
    "10/12" => "83.333333%",
    "11/12" => "91.666667%",
    "screen" => "100vw",
    "svw" => "100svw",
    "lvw" => "100lvw",
    "dvw" => "100dvw",
    "min" => "min-content",
    "max" => "max-content",
    "fit" => "fit-content",
};

/// Font size scale
static FONT_SIZE_SCALE: phf::Map<&'static str, (&'static str, &'static str)> = phf_map! {
    "xs" => ("0.75rem", "1rem"),
    "sm" => ("0.875rem", "1.25rem"),
    "base" => ("1rem", "1.5rem"),
    "lg" => ("1.125rem", "1.75rem"),
    "xl" => ("1.25rem", "1.75rem"),
    "2xl" => ("1.5rem", "2rem"),
    "3xl" => ("1.875rem", "2.25rem"),
    "4xl" => ("2.25rem", "2.5rem"),
    "5xl" => ("3rem", "1"),
    "6xl" => ("3.75rem", "1"),
    "7xl" => ("4.5rem", "1"),
    "8xl" => ("6rem", "1"),
    "9xl" => ("8rem", "1"),
};

/// Font weight scale
static FONT_WEIGHT_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "thin" => "100",
    "extralight" => "200",
    "light" => "300",
    "normal" => "400",
    "medium" => "500",
    "semibold" => "600",
    "bold" => "700",
    "extrabold" => "800",
    "black" => "900",
};

/// Border radius scale
static BORDER_RADIUS_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "none" => "0px",
    "sm" => "0.125rem",
    "" => "0.25rem",
    "md" => "0.375rem",
    "lg" => "0.5rem",
    "xl" => "0.75rem",
    "2xl" => "1rem",
    "3xl" => "1.5rem",
    "full" => "9999px",
};

/// Opacity scale
static OPACITY_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0",
    "5" => "0.05",
    "10" => "0.1",
    "15" => "0.15",
    "20" => "0.2",
    "25" => "0.25",
    "30" => "0.3",
    "35" => "0.35",
    "40" => "0.4",
    "45" => "0.45",
    "50" => "0.5",
    "55" => "0.55",
    "60" => "0.6",
    "65" => "0.65",
    "70" => "0.7",
    "75" => "0.75",
    "80" => "0.8",
    "85" => "0.85",
    "90" => "0.9",
    "95" => "0.95",
    "100" => "1",
};

/// Z-index scale
static Z_INDEX_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0",
    "10" => "10",
    "20" => "20",
    "30" => "30",
    "40" => "40",
    "50" => "50",
    "auto" => "auto",
};

/// Box shadow scale
static BOX_SHADOW_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "sm" => "0 1px 2px 0 rgb(0 0 0 / 0.05)",
    "" => "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
    "md" => "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
    "lg" => "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
    "xl" => "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)",
    "2xl" => "0 25px 50px -12px rgb(0 0 0 / 0.25)",
    "inner" => "inset 0 2px 4px 0 rgb(0 0 0 / 0.05)",
    "none" => "0 0 #0000",
};

/// Border width scale
static BORDER_WIDTH_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0px",
    "" => "1px",
    "2" => "2px",
    "4" => "4px",
    "8" => "8px",
};

/// Transition duration scale
static DURATION_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0s",
    "75" => "75ms",
    "100" => "100ms",
    "150" => "150ms",
    "200" => "200ms",
    "300" => "300ms",
    "500" => "500ms",
    "700" => "700ms",
    "1000" => "1000ms",
};

/// Ease timing functions
static EASE_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "linear" => "linear",
    "in" => "cubic-bezier(0.4, 0, 1, 1)",
    "out" => "cubic-bezier(0, 0, 0.2, 1)",
    "in-out" => "cubic-bezier(0.4, 0, 0.2, 1)",
};

/// Check if a string contains Tailwind classes
pub fn has_tailwind_classes(class_str: &str) -> bool {
    // Simple heuristic: if it looks like a Tailwind class pattern
    let parts: Vec<&str> = class_str.split_whitespace().collect();
    for part in parts {
        if is_likely_tailwind_class(part) {
            return true;
        }
    }
    false
}

/// Check if a single class looks like a Tailwind utility
fn is_likely_tailwind_class(class: &str) -> bool {
    // Strip any responsive/variant prefixes
    let class = class
        .split(':')
        .next_back()
        .unwrap_or(class)
        .trim_start_matches('-');

    // Common Tailwind prefixes
    let prefixes = [
        "bg-",
        "text-",
        "font-",
        "p-",
        "px-",
        "py-",
        "pt-",
        "pr-",
        "pb-",
        "pl-",
        "m-",
        "mx-",
        "my-",
        "mt-",
        "mr-",
        "mb-",
        "ml-",
        "w-",
        "h-",
        "min-w-",
        "max-w-",
        "min-h-",
        "max-h-",
        "flex",
        "grid",
        "block",
        "inline",
        "hidden",
        "absolute",
        "relative",
        "fixed",
        "sticky",
        "top-",
        "right-",
        "bottom-",
        "left-",
        "inset-",
        "z-",
        "opacity-",
        "rounded",
        "border",
        "shadow",
        "gap-",
        "space-",
        "items-",
        "justify-",
        "content-",
        "self-",
        "order-",
        "col-",
        "row-",
        "overflow-",
        "object-",
        "aspect-",
        "transition",
        "duration-",
        "ease-",
        "delay-",
        "animate-",
        "cursor-",
        "select-",
        "resize",
        "appearance-",
        "outline",
        "ring",
        "fill-",
        "stroke-",
        "sr-",
        "not-sr-",
        "container",
        "columns-",
        "break-",
        "decoration-",
        "underline",
        "overline",
        "line-through",
        "no-underline",
        "uppercase",
        "lowercase",
        "capitalize",
        "normal-case",
        "truncate",
        "leading-",
        "tracking-",
        "list-",
        "align-",
        "whitespace-",
        "hyphens-",
        "blur",
        "brightness-",
        "contrast-",
        "grayscale",
        "invert",
        "saturate-",
        "sepia",
        "drop-shadow",
        "backdrop-",
        "scale-",
        "rotate-",
        "translate-",
        "skew-",
        "origin-",
        "accent-",
        "caret-",
        "scroll-",
        "snap-",
        "touch-",
        "will-change-",
        "table",
        "clear-",
        "float-",
        "isolate",
        "isolation-",
        "mix-blend-",
        "bg-blend-",
        "divide-",
        "place-",
        "grow",
        "shrink",
        "basis-",
    ];

    // Exact matches for utility classes without values
    let exact_matches = [
        "flex",
        "inline-flex",
        "grid",
        "inline-grid",
        "block",
        "inline-block",
        "inline",
        "contents",
        "flow-root",
        "hidden",
        "invisible",
        "visible",
        "collapse",
        "absolute",
        "relative",
        "fixed",
        "sticky",
        "static",
        "isolate",
        "isolation-auto",
        "container",
        "truncate",
        "uppercase",
        "lowercase",
        "capitalize",
        "normal-case",
        "italic",
        "not-italic",
        "underline",
        "overline",
        "line-through",
        "no-underline",
        "antialiased",
        "subpixel-antialiased",
        "ordinal",
        "slashed-zero",
        "lining-nums",
        "oldstyle-nums",
        "proportional-nums",
        "tabular-nums",
        "diagonal-fractions",
        "stacked-fractions",
        "sr-only",
        "not-sr-only",
        "resize",
        "resize-none",
        "resize-y",
        "resize-x",
        "transition",
        "transition-none",
        "transition-all",
        "transition-colors",
        "transition-opacity",
        "transition-shadow",
        "transition-transform",
        "animate-none",
        "animate-spin",
        "animate-ping",
        "animate-pulse",
        "animate-bounce",
        "grayscale",
        "grayscale-0",
        "invert",
        "invert-0",
        "sepia",
        "sepia-0",
        "backdrop-blur",
        "backdrop-blur-none",
        "backdrop-grayscale",
        "backdrop-grayscale-0",
        "backdrop-invert",
        "backdrop-invert-0",
        "backdrop-sepia",
        "backdrop-sepia-0",
        "table",
        "table-caption",
        "table-cell",
        "table-column",
        "table-column-group",
        "table-footer-group",
        "table-header-group",
        "table-row-group",
        "table-row",
        "border-collapse",
        "border-separate",
        "grow",
        "grow-0",
        "shrink",
        "shrink-0",
        "rounded",
        "rounded-none",
        "rounded-sm",
        "rounded-md",
        "rounded-lg",
        "rounded-xl",
        "rounded-2xl",
        "rounded-3xl",
        "rounded-full",
        "border",
        "border-0",
        "border-2",
        "border-4",
        "border-8",
        "shadow",
        "shadow-sm",
        "shadow-md",
        "shadow-lg",
        "shadow-xl",
        "shadow-2xl",
        "shadow-inner",
        "shadow-none",
        "outline",
        "outline-none",
        "outline-dashed",
        "outline-dotted",
        "outline-double",
        "ring",
        "ring-0",
        "ring-1",
        "ring-2",
        "ring-4",
        "ring-8",
        "ring-inset",
        "blur",
        "blur-none",
        "blur-sm",
        "blur-md",
        "blur-lg",
        "blur-xl",
        "blur-2xl",
        "blur-3xl",
    ];

    if exact_matches.contains(&class) {
        return true;
    }

    for prefix in prefixes {
        if let Some(value_part) = class.strip_prefix(prefix) {
            // For prefixes that end with '-', validate the value part
            if prefix.ends_with('-') {
                if is_valid_tailwind_value(value_part) {
                    return true;
                }
            } else {
                // Prefix without dash (like "flex", "grid") - exact prefix match is enough
                return true;
            }
        }
    }

    // Check for arbitrary value syntax
    if class.contains('[') && class.contains(']') {
        return true;
    }

    false
}

/// Check if a value part looks like a valid Tailwind value
fn is_valid_tailwind_value(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    // Arbitrary value syntax [...]
    if value.starts_with('[') && value.ends_with(']') {
        return true;
    }

    // Common keywords
    let keywords = [
        "auto",
        "full",
        "screen",
        "min",
        "max",
        "fit",
        "px",
        "none",
        "inherit",
        "current",
        "transparent",
        "black",
        "white",
    ];
    if keywords.contains(&value) {
        return true;
    }

    // Numeric values (including decimals like 0.5, 1.5)
    let first_char = value.chars().next().unwrap();
    if first_char.is_ascii_digit() {
        return true;
    }

    // Color names with shade (e.g., red-500, blue-100)
    let color_names = [
        "slate", "gray", "zinc", "neutral", "stone", "red", "orange", "amber", "yellow", "lime",
        "green", "emerald", "teal", "cyan", "sky", "blue", "indigo", "violet", "purple", "fuchsia",
        "pink", "rose",
    ];
    for color in color_names {
        if let Some(rest) = value.strip_prefix(color) {
            // Must be followed by nothing or a dash and number
            if rest.is_empty() || rest.starts_with('-') {
                return true;
            }
        }
    }

    // Size suffixes (xs, sm, md, lg, xl, 2xl, etc.)
    let size_keywords = [
        "xs", "sm", "md", "lg", "xl", "2xl", "3xl", "4xl", "5xl", "6xl", "7xl",
    ];
    if size_keywords.contains(&value) {
        return true;
    }

    // Fraction values (1/2, 1/3, 2/3, etc.)
    if value.contains('/') {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() == 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
    }

    false
}

/// Parse a className string into a list of ExtractStyleValue
pub fn parse_tailwind_to_styles(class_str: &str, filename: Option<&str>) -> Vec<ExtractStyleValue> {
    let mut styles = Vec::new();

    for class in class_str.split_whitespace() {
        if let Some(parsed) = parse_single_class(class) {
            let static_style = parsed.to_static_style();
            styles.push(ExtractStyleValue::Static(static_style));
        }
    }

    // Set filename for all styles if provided
    if filename.is_some() {
        // The filename is already used in ExtractStaticStyle through the extract() method
    }

    styles
}

/// Parse a single Tailwind class string
pub fn parse_single_class(class: &str) -> Option<TailwindClass> {
    let mut remaining = class;
    let mut responsive_level: u8 = 0;
    let mut variants: Vec<TailwindVariant> = Vec::new();

    // Handle negative prefix at the start
    let negative = remaining.starts_with('-');
    if negative {
        remaining = &remaining[1..];
    }

    // Parse prefixes (responsive and variants)
    while let Some(colon_pos) = remaining.find(':') {
        let prefix = &remaining[..colon_pos];

        // Check if it's a responsive prefix
        if let Some(&level) = RESPONSIVE_PREFIX_MAP.get(prefix) {
            responsive_level = level;
        } else if let Some(variant) = TailwindVariant::from_prefix(prefix) {
            variants.push(variant);
        } else {
            // Unknown prefix, might be an arbitrary variant
            // For now, skip unknown prefixes
        }

        remaining = &remaining[colon_pos + 1..];
    }

    // Now parse the utility class
    parse_utility(remaining, negative).map(|(property, value)| TailwindClass {
        responsive: responsive_level,
        variants,
        property,
        value,
        negative,
    })
}

/// Parse a utility class (without prefixes) into property and value
fn parse_utility(class: &str, is_negative: bool) -> Option<(String, String)> {
    // Handle arbitrary values first
    if let Some(result) = parse_arbitrary_value(class) {
        return Some(result);
    }

    // Layout utilities
    if let Some(result) = parse_layout_utility(class) {
        return Some(result);
    }

    // Flexbox & Grid
    if let Some(result) = parse_flex_grid_utility(class) {
        return Some(result);
    }

    // Spacing (padding, margin)
    if let Some(result) = parse_spacing_utility(class, is_negative) {
        return Some(result);
    }

    // Sizing (width, height)
    if let Some(result) = parse_sizing_utility(class) {
        return Some(result);
    }

    // Typography
    if let Some(result) = parse_typography_utility(class) {
        return Some(result);
    }

    // Backgrounds
    if let Some(result) = parse_background_utility(class) {
        return Some(result);
    }

    // Borders
    if let Some(result) = parse_border_utility(class) {
        return Some(result);
    }

    // Effects (shadow, opacity)
    if let Some(result) = parse_effects_utility(class) {
        return Some(result);
    }

    // Filters
    if let Some(result) = parse_filter_utility(class) {
        return Some(result);
    }

    // Transitions & Animation
    if let Some(result) = parse_transition_utility(class) {
        return Some(result);
    }

    // Transforms
    if let Some(result) = parse_transform_utility(class, is_negative) {
        return Some(result);
    }

    // Interactivity
    if let Some(result) = parse_interactivity_utility(class) {
        return Some(result);
    }

    // SVG
    if let Some(result) = parse_svg_utility(class) {
        return Some(result);
    }

    // Accessibility
    if let Some(result) = parse_accessibility_utility(class) {
        return Some(result);
    }

    None
}

/// Parse arbitrary value syntax: class-[value]
fn parse_arbitrary_value(class: &str) -> Option<(String, String)> {
    if !class.contains('[') {
        return None;
    }

    let bracket_start = class.find('[')?;
    let bracket_end = class.rfind(']')?;

    if bracket_end <= bracket_start {
        return None;
    }

    let prefix = &class[..bracket_start];
    let value = &class[bracket_start + 1..bracket_end];

    // Replace underscores with spaces in arbitrary values
    let value = value.replace('_', " ");

    match prefix {
        "w-" => Some(("width".to_string(), value)),
        "h-" => Some(("height".to_string(), value)),
        "min-w-" => Some(("min-width".to_string(), value)),
        "max-w-" => Some(("max-width".to_string(), value)),
        "min-h-" => Some(("min-height".to_string(), value)),
        "max-h-" => Some(("max-height".to_string(), value)),
        "p-" => Some(("padding".to_string(), value)),
        "px-" => Some(("padding-inline".to_string(), value)),
        "py-" => Some(("padding-block".to_string(), value)),
        "pt-" => Some(("padding-top".to_string(), value)),
        "pr-" => Some(("padding-right".to_string(), value)),
        "pb-" => Some(("padding-bottom".to_string(), value)),
        "pl-" => Some(("padding-left".to_string(), value)),
        "m-" => Some(("margin".to_string(), value)),
        "mx-" => Some(("margin-inline".to_string(), value)),
        "my-" => Some(("margin-block".to_string(), value)),
        "mt-" => Some(("margin-top".to_string(), value)),
        "mr-" => Some(("margin-right".to_string(), value)),
        "mb-" => Some(("margin-bottom".to_string(), value)),
        "ml-" => Some(("margin-left".to_string(), value)),
        "top-" => Some(("top".to_string(), value)),
        "right-" => Some(("right".to_string(), value)),
        "bottom-" => Some(("bottom".to_string(), value)),
        "left-" => Some(("left".to_string(), value)),
        "inset-" => Some(("inset".to_string(), value)),
        "inset-x-" => Some(("inset-inline".to_string(), value)),
        "inset-y-" => Some(("inset-block".to_string(), value)),
        "gap-" => Some(("gap".to_string(), value)),
        "gap-x-" => Some(("column-gap".to_string(), value)),
        "gap-y-" => Some(("row-gap".to_string(), value)),
        "text-" => Some(("color".to_string(), value)),
        "bg-" => Some(("background-color".to_string(), value)),
        "border-" => Some(("border-color".to_string(), value)),
        "rounded-" => Some(("border-radius".to_string(), value)),
        "opacity-" => Some(("opacity".to_string(), value)),
        "z-" => Some(("z-index".to_string(), value)),
        "font-" => Some(("font-family".to_string(), value)),
        "tracking-" => Some(("letter-spacing".to_string(), value)),
        "leading-" => Some(("line-height".to_string(), value)),
        "duration-" => Some(("transition-duration".to_string(), value)),
        "delay-" => Some(("transition-delay".to_string(), value)),
        "scale-" => Some(("transform".to_string(), format!("scale({})", value))),
        "rotate-" => Some(("transform".to_string(), format!("rotate({})", value))),
        "translate-x-" => Some(("transform".to_string(), format!("translateX({})", value))),
        "translate-y-" => Some(("transform".to_string(), format!("translateY({})", value))),
        "skew-x-" => Some(("transform".to_string(), format!("skewX({})", value))),
        "skew-y-" => Some(("transform".to_string(), format!("skewY({})", value))),
        "aspect-" => Some(("aspect-ratio".to_string(), value)),
        "columns-" => Some(("columns".to_string(), value)),
        "grid-cols-" => Some((
            "grid-template-columns".to_string(),
            format!("repeat({}, minmax(0, 1fr))", value),
        )),
        "grid-rows-" => Some((
            "grid-template-rows".to_string(),
            format!("repeat({}, minmax(0, 1fr))", value),
        )),
        "col-span-" => Some((
            "grid-column".to_string(),
            format!("span {} / span {}", value, value),
        )),
        "row-span-" => Some((
            "grid-row".to_string(),
            format!("span {} / span {}", value, value),
        )),
        "basis-" => Some(("flex-basis".to_string(), value)),
        "blur-" => Some(("filter".to_string(), format!("blur({})", value))),
        "brightness-" => Some(("filter".to_string(), format!("brightness({})", value))),
        "contrast-" => Some(("filter".to_string(), format!("contrast({})", value))),
        "saturate-" => Some(("filter".to_string(), format!("saturate({})", value))),
        "backdrop-blur-" => Some(("backdrop-filter".to_string(), format!("blur({})", value))),
        _ => None,
    }
}

/// Parse layout utilities (display, position, visibility, etc.)
fn parse_layout_utility(class: &str) -> Option<(String, String)> {
    match class {
        // Display
        "block" => Some(("display".to_string(), "block".to_string())),
        "inline-block" => Some(("display".to_string(), "inline-block".to_string())),
        "inline" => Some(("display".to_string(), "inline".to_string())),
        "flex" => Some(("display".to_string(), "flex".to_string())),
        "inline-flex" => Some(("display".to_string(), "inline-flex".to_string())),
        "table" => Some(("display".to_string(), "table".to_string())),
        "inline-table" => Some(("display".to_string(), "inline-table".to_string())),
        "table-caption" => Some(("display".to_string(), "table-caption".to_string())),
        "table-cell" => Some(("display".to_string(), "table-cell".to_string())),
        "table-column" => Some(("display".to_string(), "table-column".to_string())),
        "table-column-group" => Some(("display".to_string(), "table-column-group".to_string())),
        "table-footer-group" => Some(("display".to_string(), "table-footer-group".to_string())),
        "table-header-group" => Some(("display".to_string(), "table-header-group".to_string())),
        "table-row-group" => Some(("display".to_string(), "table-row-group".to_string())),
        "table-row" => Some(("display".to_string(), "table-row".to_string())),
        "flow-root" => Some(("display".to_string(), "flow-root".to_string())),
        "grid" => Some(("display".to_string(), "grid".to_string())),
        "inline-grid" => Some(("display".to_string(), "inline-grid".to_string())),
        "contents" => Some(("display".to_string(), "contents".to_string())),
        "list-item" => Some(("display".to_string(), "list-item".to_string())),
        "hidden" => Some(("display".to_string(), "none".to_string())),

        // Position
        "static" => Some(("position".to_string(), "static".to_string())),
        "fixed" => Some(("position".to_string(), "fixed".to_string())),
        "absolute" => Some(("position".to_string(), "absolute".to_string())),
        "relative" => Some(("position".to_string(), "relative".to_string())),
        "sticky" => Some(("position".to_string(), "sticky".to_string())),

        // Visibility
        "visible" => Some(("visibility".to_string(), "visible".to_string())),
        "invisible" => Some(("visibility".to_string(), "hidden".to_string())),
        "collapse" => Some(("visibility".to_string(), "collapse".to_string())),

        // Box sizing
        "box-border" => Some(("box-sizing".to_string(), "border-box".to_string())),
        "box-content" => Some(("box-sizing".to_string(), "content-box".to_string())),

        // Float
        "float-start" => Some(("float".to_string(), "inline-start".to_string())),
        "float-end" => Some(("float".to_string(), "inline-end".to_string())),
        "float-right" => Some(("float".to_string(), "right".to_string())),
        "float-left" => Some(("float".to_string(), "left".to_string())),
        "float-none" => Some(("float".to_string(), "none".to_string())),

        // Clear
        "clear-start" => Some(("clear".to_string(), "inline-start".to_string())),
        "clear-end" => Some(("clear".to_string(), "inline-end".to_string())),
        "clear-left" => Some(("clear".to_string(), "left".to_string())),
        "clear-right" => Some(("clear".to_string(), "right".to_string())),
        "clear-both" => Some(("clear".to_string(), "both".to_string())),
        "clear-none" => Some(("clear".to_string(), "none".to_string())),

        // Isolation
        "isolate" => Some(("isolation".to_string(), "isolate".to_string())),
        "isolation-auto" => Some(("isolation".to_string(), "auto".to_string())),

        // Object fit
        "object-contain" => Some(("object-fit".to_string(), "contain".to_string())),
        "object-cover" => Some(("object-fit".to_string(), "cover".to_string())),
        "object-fill" => Some(("object-fit".to_string(), "fill".to_string())),
        "object-none" => Some(("object-fit".to_string(), "none".to_string())),
        "object-scale-down" => Some(("object-fit".to_string(), "scale-down".to_string())),

        // Object position
        "object-bottom" => Some(("object-position".to_string(), "bottom".to_string())),
        "object-center" => Some(("object-position".to_string(), "center".to_string())),
        "object-left" => Some(("object-position".to_string(), "left".to_string())),
        "object-left-bottom" => Some(("object-position".to_string(), "left bottom".to_string())),
        "object-left-top" => Some(("object-position".to_string(), "left top".to_string())),
        "object-right" => Some(("object-position".to_string(), "right".to_string())),
        "object-right-bottom" => Some(("object-position".to_string(), "right bottom".to_string())),
        "object-right-top" => Some(("object-position".to_string(), "right top".to_string())),
        "object-top" => Some(("object-position".to_string(), "top".to_string())),

        // Overflow
        "overflow-auto" => Some(("overflow".to_string(), "auto".to_string())),
        "overflow-hidden" => Some(("overflow".to_string(), "hidden".to_string())),
        "overflow-clip" => Some(("overflow".to_string(), "clip".to_string())),
        "overflow-visible" => Some(("overflow".to_string(), "visible".to_string())),
        "overflow-scroll" => Some(("overflow".to_string(), "scroll".to_string())),
        "overflow-x-auto" => Some(("overflow-x".to_string(), "auto".to_string())),
        "overflow-y-auto" => Some(("overflow-y".to_string(), "auto".to_string())),
        "overflow-x-hidden" => Some(("overflow-x".to_string(), "hidden".to_string())),
        "overflow-y-hidden" => Some(("overflow-y".to_string(), "hidden".to_string())),
        "overflow-x-clip" => Some(("overflow-x".to_string(), "clip".to_string())),
        "overflow-y-clip" => Some(("overflow-y".to_string(), "clip".to_string())),
        "overflow-x-visible" => Some(("overflow-x".to_string(), "visible".to_string())),
        "overflow-y-visible" => Some(("overflow-y".to_string(), "visible".to_string())),
        "overflow-x-scroll" => Some(("overflow-x".to_string(), "scroll".to_string())),
        "overflow-y-scroll" => Some(("overflow-y".to_string(), "scroll".to_string())),

        // Overscroll
        "overscroll-auto" => Some(("overscroll-behavior".to_string(), "auto".to_string())),
        "overscroll-contain" => Some(("overscroll-behavior".to_string(), "contain".to_string())),
        "overscroll-none" => Some(("overscroll-behavior".to_string(), "none".to_string())),
        "overscroll-x-auto" => Some(("overscroll-behavior-x".to_string(), "auto".to_string())),
        "overscroll-x-contain" => {
            Some(("overscroll-behavior-x".to_string(), "contain".to_string()))
        }
        "overscroll-x-none" => Some(("overscroll-behavior-x".to_string(), "none".to_string())),
        "overscroll-y-auto" => Some(("overscroll-behavior-y".to_string(), "auto".to_string())),
        "overscroll-y-contain" => {
            Some(("overscroll-behavior-y".to_string(), "contain".to_string()))
        }
        "overscroll-y-none" => Some(("overscroll-behavior-y".to_string(), "none".to_string())),

        _ => {
            // Aspect ratio
            if let Some(rest) = class.strip_prefix("aspect-") {
                let value = match rest {
                    "auto" => "auto".to_string(),
                    "square" => "1 / 1".to_string(),
                    "video" => "16 / 9".to_string(),
                    v => v.replace('-', " / "),
                };
                return Some(("aspect-ratio".to_string(), value));
            }

            // Columns
            if let Some(rest) = class.strip_prefix("columns-") {
                let value = match rest {
                    "auto" => "auto".to_string(),
                    "3xs" => "16rem".to_string(),
                    "2xs" => "18rem".to_string(),
                    "xs" => "20rem".to_string(),
                    "sm" => "24rem".to_string(),
                    "md" => "28rem".to_string(),
                    "lg" => "32rem".to_string(),
                    "xl" => "36rem".to_string(),
                    "2xl" => "42rem".to_string(),
                    "3xl" => "48rem".to_string(),
                    "4xl" => "56rem".to_string(),
                    "5xl" => "64rem".to_string(),
                    "6xl" => "72rem".to_string(),
                    "7xl" => "80rem".to_string(),
                    v => v.to_string(),
                };
                return Some(("columns".to_string(), value));
            }

            // Break utilities
            if let Some(rest) = class.strip_prefix("break-") {
                return match rest {
                    "after-auto" => Some(("break-after".to_string(), "auto".to_string())),
                    "after-avoid" => Some(("break-after".to_string(), "avoid".to_string())),
                    "after-all" => Some(("break-after".to_string(), "all".to_string())),
                    "after-avoid-page" => {
                        Some(("break-after".to_string(), "avoid-page".to_string()))
                    }
                    "after-page" => Some(("break-after".to_string(), "page".to_string())),
                    "after-left" => Some(("break-after".to_string(), "left".to_string())),
                    "after-right" => Some(("break-after".to_string(), "right".to_string())),
                    "after-column" => Some(("break-after".to_string(), "column".to_string())),
                    "before-auto" => Some(("break-before".to_string(), "auto".to_string())),
                    "before-avoid" => Some(("break-before".to_string(), "avoid".to_string())),
                    "before-all" => Some(("break-before".to_string(), "all".to_string())),
                    "before-avoid-page" => {
                        Some(("break-before".to_string(), "avoid-page".to_string()))
                    }
                    "before-page" => Some(("break-before".to_string(), "page".to_string())),
                    "before-left" => Some(("break-before".to_string(), "left".to_string())),
                    "before-right" => Some(("break-before".to_string(), "right".to_string())),
                    "before-column" => Some(("break-before".to_string(), "column".to_string())),
                    "inside-auto" => Some(("break-inside".to_string(), "auto".to_string())),
                    "inside-avoid" => Some(("break-inside".to_string(), "avoid".to_string())),
                    "inside-avoid-page" => {
                        Some(("break-inside".to_string(), "avoid-page".to_string()))
                    }
                    "inside-avoid-column" => {
                        Some(("break-inside".to_string(), "avoid-column".to_string()))
                    }
                    _ => None,
                };
            }

            // Box decoration break
            if class == "box-decoration-clone" {
                return Some(("box-decoration-break".to_string(), "clone".to_string()));
            }
            if class == "box-decoration-slice" {
                return Some(("box-decoration-break".to_string(), "slice".to_string()));
            }

            // Z-index
            if let Some(rest) = class.strip_prefix("z-") {
                if let Some(&value) = Z_INDEX_SCALE.get(rest) {
                    return Some(("z-index".to_string(), value.to_string()));
                }
            }

            // Top/Right/Bottom/Left/Inset
            if let Some(rest) = class.strip_prefix("top-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("top".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("right-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("right".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("bottom-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("bottom".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("left-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("left".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("inset-x-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("inset-inline".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("inset-y-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("inset-block".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("inset-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("inset".to_string(), value.to_string()));
                }
            }

            None
        }
    }
}

/// Parse flexbox and grid utilities
fn parse_flex_grid_utility(class: &str) -> Option<(String, String)> {
    match class {
        // Flex basis
        "basis-auto" => Some(("flex-basis".to_string(), "auto".to_string())),
        "basis-full" => Some(("flex-basis".to_string(), "100%".to_string())),

        // Flex direction
        "flex-row" => Some(("flex-direction".to_string(), "row".to_string())),
        "flex-row-reverse" => Some(("flex-direction".to_string(), "row-reverse".to_string())),
        "flex-col" => Some(("flex-direction".to_string(), "column".to_string())),
        "flex-col-reverse" => Some(("flex-direction".to_string(), "column-reverse".to_string())),

        // Flex wrap
        "flex-wrap" => Some(("flex-wrap".to_string(), "wrap".to_string())),
        "flex-wrap-reverse" => Some(("flex-wrap".to_string(), "wrap-reverse".to_string())),
        "flex-nowrap" => Some(("flex-wrap".to_string(), "nowrap".to_string())),

        // Flex
        "flex-1" => Some(("flex".to_string(), "1 1 0%".to_string())),
        "flex-auto" => Some(("flex".to_string(), "1 1 auto".to_string())),
        "flex-initial" => Some(("flex".to_string(), "0 1 auto".to_string())),
        "flex-none" => Some(("flex".to_string(), "none".to_string())),

        // Grow/Shrink
        "grow" => Some(("flex-grow".to_string(), "1".to_string())),
        "grow-0" => Some(("flex-grow".to_string(), "0".to_string())),
        "shrink" => Some(("flex-shrink".to_string(), "1".to_string())),
        "shrink-0" => Some(("flex-shrink".to_string(), "0".to_string())),

        // Order
        "order-first" => Some(("order".to_string(), "-9999".to_string())),
        "order-last" => Some(("order".to_string(), "9999".to_string())),
        "order-none" => Some(("order".to_string(), "0".to_string())),

        // Grid template columns
        "grid-cols-none" => Some(("grid-template-columns".to_string(), "none".to_string())),
        "grid-cols-subgrid" => Some(("grid-template-columns".to_string(), "subgrid".to_string())),

        // Grid template rows
        "grid-rows-none" => Some(("grid-template-rows".to_string(), "none".to_string())),
        "grid-rows-subgrid" => Some(("grid-template-rows".to_string(), "subgrid".to_string())),

        // Grid column
        "col-auto" => Some(("grid-column".to_string(), "auto".to_string())),
        "col-span-full" => Some(("grid-column".to_string(), "1 / -1".to_string())),
        "col-start-auto" => Some(("grid-column-start".to_string(), "auto".to_string())),
        "col-end-auto" => Some(("grid-column-end".to_string(), "auto".to_string())),

        // Grid row
        "row-auto" => Some(("grid-row".to_string(), "auto".to_string())),
        "row-span-full" => Some(("grid-row".to_string(), "1 / -1".to_string())),
        "row-start-auto" => Some(("grid-row-start".to_string(), "auto".to_string())),
        "row-end-auto" => Some(("grid-row-end".to_string(), "auto".to_string())),

        // Grid auto flow
        "grid-flow-row" => Some(("grid-auto-flow".to_string(), "row".to_string())),
        "grid-flow-col" => Some(("grid-auto-flow".to_string(), "column".to_string())),
        "grid-flow-dense" => Some(("grid-auto-flow".to_string(), "dense".to_string())),
        "grid-flow-row-dense" => Some(("grid-auto-flow".to_string(), "row dense".to_string())),
        "grid-flow-col-dense" => Some(("grid-auto-flow".to_string(), "column dense".to_string())),

        // Grid auto columns
        "auto-cols-auto" => Some(("grid-auto-columns".to_string(), "auto".to_string())),
        "auto-cols-min" => Some(("grid-auto-columns".to_string(), "min-content".to_string())),
        "auto-cols-max" => Some(("grid-auto-columns".to_string(), "max-content".to_string())),
        "auto-cols-fr" => Some((
            "grid-auto-columns".to_string(),
            "minmax(0, 1fr)".to_string(),
        )),

        // Grid auto rows
        "auto-rows-auto" => Some(("grid-auto-rows".to_string(), "auto".to_string())),
        "auto-rows-min" => Some(("grid-auto-rows".to_string(), "min-content".to_string())),
        "auto-rows-max" => Some(("grid-auto-rows".to_string(), "max-content".to_string())),
        "auto-rows-fr" => Some(("grid-auto-rows".to_string(), "minmax(0, 1fr)".to_string())),

        // Justify content
        "justify-normal" => Some(("justify-content".to_string(), "normal".to_string())),
        "justify-start" => Some(("justify-content".to_string(), "flex-start".to_string())),
        "justify-end" => Some(("justify-content".to_string(), "flex-end".to_string())),
        "justify-center" => Some(("justify-content".to_string(), "center".to_string())),
        "justify-between" => Some(("justify-content".to_string(), "space-between".to_string())),
        "justify-around" => Some(("justify-content".to_string(), "space-around".to_string())),
        "justify-evenly" => Some(("justify-content".to_string(), "space-evenly".to_string())),
        "justify-stretch" => Some(("justify-content".to_string(), "stretch".to_string())),

        // Justify items
        "justify-items-start" => Some(("justify-items".to_string(), "start".to_string())),
        "justify-items-end" => Some(("justify-items".to_string(), "end".to_string())),
        "justify-items-center" => Some(("justify-items".to_string(), "center".to_string())),
        "justify-items-stretch" => Some(("justify-items".to_string(), "stretch".to_string())),

        // Justify self
        "justify-self-auto" => Some(("justify-self".to_string(), "auto".to_string())),
        "justify-self-start" => Some(("justify-self".to_string(), "start".to_string())),
        "justify-self-end" => Some(("justify-self".to_string(), "end".to_string())),
        "justify-self-center" => Some(("justify-self".to_string(), "center".to_string())),
        "justify-self-stretch" => Some(("justify-self".to_string(), "stretch".to_string())),

        // Align content
        "content-normal" => Some(("align-content".to_string(), "normal".to_string())),
        "content-center" => Some(("align-content".to_string(), "center".to_string())),
        "content-start" => Some(("align-content".to_string(), "flex-start".to_string())),
        "content-end" => Some(("align-content".to_string(), "flex-end".to_string())),
        "content-between" => Some(("align-content".to_string(), "space-between".to_string())),
        "content-around" => Some(("align-content".to_string(), "space-around".to_string())),
        "content-evenly" => Some(("align-content".to_string(), "space-evenly".to_string())),
        "content-baseline" => Some(("align-content".to_string(), "baseline".to_string())),
        "content-stretch" => Some(("align-content".to_string(), "stretch".to_string())),

        // Align items
        "items-start" => Some(("align-items".to_string(), "flex-start".to_string())),
        "items-end" => Some(("align-items".to_string(), "flex-end".to_string())),
        "items-center" => Some(("align-items".to_string(), "center".to_string())),
        "items-baseline" => Some(("align-items".to_string(), "baseline".to_string())),
        "items-stretch" => Some(("align-items".to_string(), "stretch".to_string())),

        // Align self
        "self-auto" => Some(("align-self".to_string(), "auto".to_string())),
        "self-start" => Some(("align-self".to_string(), "flex-start".to_string())),
        "self-end" => Some(("align-self".to_string(), "flex-end".to_string())),
        "self-center" => Some(("align-self".to_string(), "center".to_string())),
        "self-stretch" => Some(("align-self".to_string(), "stretch".to_string())),
        "self-baseline" => Some(("align-self".to_string(), "baseline".to_string())),

        // Place content
        "place-content-center" => Some(("place-content".to_string(), "center".to_string())),
        "place-content-start" => Some(("place-content".to_string(), "start".to_string())),
        "place-content-end" => Some(("place-content".to_string(), "end".to_string())),
        "place-content-between" => Some(("place-content".to_string(), "space-between".to_string())),
        "place-content-around" => Some(("place-content".to_string(), "space-around".to_string())),
        "place-content-evenly" => Some(("place-content".to_string(), "space-evenly".to_string())),
        "place-content-baseline" => Some(("place-content".to_string(), "baseline".to_string())),
        "place-content-stretch" => Some(("place-content".to_string(), "stretch".to_string())),

        // Place items
        "place-items-start" => Some(("place-items".to_string(), "start".to_string())),
        "place-items-end" => Some(("place-items".to_string(), "end".to_string())),
        "place-items-center" => Some(("place-items".to_string(), "center".to_string())),
        "place-items-baseline" => Some(("place-items".to_string(), "baseline".to_string())),
        "place-items-stretch" => Some(("place-items".to_string(), "stretch".to_string())),

        // Place self
        "place-self-auto" => Some(("place-self".to_string(), "auto".to_string())),
        "place-self-start" => Some(("place-self".to_string(), "start".to_string())),
        "place-self-end" => Some(("place-self".to_string(), "end".to_string())),
        "place-self-center" => Some(("place-self".to_string(), "center".to_string())),
        "place-self-stretch" => Some(("place-self".to_string(), "stretch".to_string())),

        _ => {
            // Flex basis with spacing scale
            if let Some(rest) = class.strip_prefix("basis-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("flex-basis".to_string(), value.to_string()));
                }
            }

            // Order with number
            if let Some(rest) = class.strip_prefix("order-") {
                return Some(("order".to_string(), rest.to_string()));
            }

            // Grid cols
            if let Some(rest) = class.strip_prefix("grid-cols-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some((
                        "grid-template-columns".to_string(),
                        format!("repeat({}, minmax(0, 1fr))", n),
                    ));
                }
            }

            // Grid rows
            if let Some(rest) = class.strip_prefix("grid-rows-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some((
                        "grid-template-rows".to_string(),
                        format!("repeat({}, minmax(0, 1fr))", n),
                    ));
                }
            }

            // Col span
            if let Some(rest) = class.strip_prefix("col-span-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some((
                        "grid-column".to_string(),
                        format!("span {} / span {}", n, n),
                    ));
                }
            }

            // Col start
            if let Some(rest) = class.strip_prefix("col-start-") {
                return Some(("grid-column-start".to_string(), rest.to_string()));
            }

            // Col end
            if let Some(rest) = class.strip_prefix("col-end-") {
                return Some(("grid-column-end".to_string(), rest.to_string()));
            }

            // Row span
            if let Some(rest) = class.strip_prefix("row-span-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some(("grid-row".to_string(), format!("span {} / span {}", n, n)));
                }
            }

            // Row start
            if let Some(rest) = class.strip_prefix("row-start-") {
                return Some(("grid-row-start".to_string(), rest.to_string()));
            }

            // Row end
            if let Some(rest) = class.strip_prefix("row-end-") {
                return Some(("grid-row-end".to_string(), rest.to_string()));
            }

            // Gap
            if let Some(rest) = class.strip_prefix("gap-x-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("column-gap".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("gap-y-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("row-gap".to_string(), value.to_string()));
                }
            }
            if let Some(rest) = class.strip_prefix("gap-") {
                if let Some(&value) = SPACING_SCALE.get(rest) {
                    return Some(("gap".to_string(), value.to_string()));
                }
            }

            None
        }
    }
}

/// Parse spacing utilities (padding, margin, space)
fn parse_spacing_utility(class: &str, _is_negative: bool) -> Option<(String, String)> {
    // Note: is_negative is handled at a higher level in TailwindClass::to_static_style()
    // We don't apply the negative sign here to avoid double-negation

    // Padding
    if let Some(rest) = class.strip_prefix("px-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-inline".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("py-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-block".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("pt-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-top".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("pr-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-right".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("pb-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-bottom".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("pl-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-left".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("ps-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-inline-start".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("pe-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding-inline-end".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("p-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("padding".to_string(), value.to_string()));
        }
    }

    // Margin
    if let Some(rest) = class.strip_prefix("mx-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-inline".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("my-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-block".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("mt-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-top".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("mr-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-right".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("mb-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-bottom".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("ml-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-left".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("ms-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-inline-start".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("me-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin-inline-end".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("m-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("margin".to_string(), value.to_string()));
        }
    }

    // Space between
    if let Some(rest) = class.strip_prefix("space-x-") {
        if rest == "reverse" {
            return Some(("--tw-space-x-reverse".to_string(), "1".to_string()));
        }
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("column-gap".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("space-y-") {
        if rest == "reverse" {
            return Some(("--tw-space-y-reverse".to_string(), "1".to_string()));
        }
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("row-gap".to_string(), value.to_string()));
        }
    }

    None
}

/// Parse sizing utilities (width, height, min/max)
fn parse_sizing_utility(class: &str) -> Option<(String, String)> {
    // Width
    if let Some(rest) = class.strip_prefix("w-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("width".to_string(), value.to_string()));
        }
    }

    // Min width
    if let Some(rest) = class.strip_prefix("min-w-") {
        let value = match rest {
            "0" => "0px".to_string(),
            "full" => "100%".to_string(),
            "min" => "min-content".to_string(),
            "max" => "max-content".to_string(),
            "fit" => "fit-content".to_string(),
            _ => {
                if let Some(&v) = SPACING_SCALE.get(rest) {
                    v.to_string()
                } else {
                    return None;
                }
            }
        };
        return Some(("min-width".to_string(), value));
    }

    // Max width
    if let Some(rest) = class.strip_prefix("max-w-") {
        let value = match rest {
            "none" => "none".to_string(),
            "0" => "0rem".to_string(),
            "xs" => "20rem".to_string(),
            "sm" => "24rem".to_string(),
            "md" => "28rem".to_string(),
            "lg" => "32rem".to_string(),
            "xl" => "36rem".to_string(),
            "2xl" => "42rem".to_string(),
            "3xl" => "48rem".to_string(),
            "4xl" => "56rem".to_string(),
            "5xl" => "64rem".to_string(),
            "6xl" => "72rem".to_string(),
            "7xl" => "80rem".to_string(),
            "full" => "100%".to_string(),
            "min" => "min-content".to_string(),
            "max" => "max-content".to_string(),
            "fit" => "fit-content".to_string(),
            "prose" => "65ch".to_string(),
            "screen-sm" => "640px".to_string(),
            "screen-md" => "768px".to_string(),
            "screen-lg" => "1024px".to_string(),
            "screen-xl" => "1280px".to_string(),
            "screen-2xl" => "1536px".to_string(),
            _ => {
                if let Some(&v) = SPACING_SCALE.get(rest) {
                    v.to_string()
                } else {
                    return None;
                }
            }
        };
        return Some(("max-width".to_string(), value));
    }

    // Height
    if let Some(rest) = class.strip_prefix("h-") {
        let value = match rest {
            "screen" => "100vh".to_string(),
            "svh" => "100svh".to_string(),
            "lvh" => "100lvh".to_string(),
            "dvh" => "100dvh".to_string(),
            _ => {
                if let Some(&v) = SPACING_SCALE.get(rest) {
                    v.to_string()
                } else {
                    return None;
                }
            }
        };
        return Some(("height".to_string(), value));
    }

    // Min height
    if let Some(rest) = class.strip_prefix("min-h-") {
        let value = match rest {
            "0" => "0px".to_string(),
            "full" => "100%".to_string(),
            "screen" => "100vh".to_string(),
            "svh" => "100svh".to_string(),
            "lvh" => "100lvh".to_string(),
            "dvh" => "100dvh".to_string(),
            "min" => "min-content".to_string(),
            "max" => "max-content".to_string(),
            "fit" => "fit-content".to_string(),
            _ => {
                if let Some(&v) = SPACING_SCALE.get(rest) {
                    v.to_string()
                } else {
                    return None;
                }
            }
        };
        return Some(("min-height".to_string(), value));
    }

    // Max height
    if let Some(rest) = class.strip_prefix("max-h-") {
        let value = match rest {
            "none" => "none".to_string(),
            "full" => "100%".to_string(),
            "screen" => "100vh".to_string(),
            "svh" => "100svh".to_string(),
            "lvh" => "100lvh".to_string(),
            "dvh" => "100dvh".to_string(),
            "min" => "min-content".to_string(),
            "max" => "max-content".to_string(),
            "fit" => "fit-content".to_string(),
            _ => {
                if let Some(&v) = SPACING_SCALE.get(rest) {
                    v.to_string()
                } else {
                    return None;
                }
            }
        };
        return Some(("max-height".to_string(), value));
    }

    // Size (width and height)
    if let Some(rest) = class.strip_prefix("size-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            // This should set both width and height
            // For simplicity, we'll use the width shorthand and handle height separately
            return Some(("width".to_string(), value.to_string()));
        }
    }

    None
}

/// Parse typography utilities
fn parse_typography_utility(class: &str) -> Option<(String, String)> {
    // Font family
    match class {
        "font-sans" => return Some(("font-family".to_string(), "ui-sans-serif, system-ui, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'".to_string())),
        "font-serif" => return Some(("font-family".to_string(), "ui-serif, Georgia, Cambria, 'Times New Roman', Times, serif".to_string())),
        "font-mono" => return Some(("font-family".to_string(), "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace".to_string())),
        _ => {}
    }

    // Font size
    if let Some(rest) = class.strip_prefix("text-") {
        // First check if it's a color
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("color".to_string(), color.to_string()));
        }
        // Then check if it's a font size
        if let Some(&(size, _line_height)) = FONT_SIZE_SCALE.get(rest) {
            // Return font-size (line-height would need separate handling)
            return Some(("font-size".to_string(), size.to_string()));
        }
        // Text alignment
        match rest {
            "left" => return Some(("text-align".to_string(), "left".to_string())),
            "center" => return Some(("text-align".to_string(), "center".to_string())),
            "right" => return Some(("text-align".to_string(), "right".to_string())),
            "justify" => return Some(("text-align".to_string(), "justify".to_string())),
            "start" => return Some(("text-align".to_string(), "start".to_string())),
            "end" => return Some(("text-align".to_string(), "end".to_string())),
            _ => {}
        }
    }

    // Font weight
    if let Some(rest) = class.strip_prefix("font-") {
        if let Some(&weight) = FONT_WEIGHT_SCALE.get(rest) {
            return Some(("font-weight".to_string(), weight.to_string()));
        }
    }

    // Font style
    match class {
        "italic" => return Some(("font-style".to_string(), "italic".to_string())),
        "not-italic" => return Some(("font-style".to_string(), "normal".to_string())),
        _ => {}
    }

    // Text decoration
    match class {
        "underline" => return Some(("text-decoration-line".to_string(), "underline".to_string())),
        "overline" => return Some(("text-decoration-line".to_string(), "overline".to_string())),
        "line-through" => {
            return Some((
                "text-decoration-line".to_string(),
                "line-through".to_string(),
            ));
        }
        "no-underline" => return Some(("text-decoration-line".to_string(), "none".to_string())),
        _ => {}
    }

    // Text transform
    match class {
        "uppercase" => return Some(("text-transform".to_string(), "uppercase".to_string())),
        "lowercase" => return Some(("text-transform".to_string(), "lowercase".to_string())),
        "capitalize" => return Some(("text-transform".to_string(), "capitalize".to_string())),
        "normal-case" => return Some(("text-transform".to_string(), "none".to_string())),
        _ => {}
    }

    // Text overflow
    match class {
        "truncate" => {
            return Some(("text-overflow".to_string(), "ellipsis".to_string()));
        }
        "text-ellipsis" => return Some(("text-overflow".to_string(), "ellipsis".to_string())),
        "text-clip" => return Some(("text-overflow".to_string(), "clip".to_string())),
        _ => {}
    }

    // Text wrap
    match class {
        "text-wrap" => return Some(("text-wrap".to_string(), "wrap".to_string())),
        "text-nowrap" => return Some(("text-wrap".to_string(), "nowrap".to_string())),
        "text-balance" => return Some(("text-wrap".to_string(), "balance".to_string())),
        "text-pretty" => return Some(("text-wrap".to_string(), "pretty".to_string())),
        _ => {}
    }

    // Whitespace
    if let Some(rest) = class.strip_prefix("whitespace-") {
        return Some(("white-space".to_string(), rest.to_string()));
    }

    // Word break
    match class {
        "break-normal" => return Some(("word-break".to_string(), "normal".to_string())),
        "break-words" => return Some(("overflow-wrap".to_string(), "break-word".to_string())),
        "break-all" => return Some(("word-break".to_string(), "break-all".to_string())),
        "break-keep" => return Some(("word-break".to_string(), "keep-all".to_string())),
        _ => {}
    }

    // Hyphens
    if let Some(rest) = class.strip_prefix("hyphens-") {
        return Some(("hyphens".to_string(), rest.to_string()));
    }

    // Letter spacing
    if let Some(rest) = class.strip_prefix("tracking-") {
        let value = match rest {
            "tighter" => "-0.05em".to_string(),
            "tight" => "-0.025em".to_string(),
            "normal" => "0em".to_string(),
            "wide" => "0.025em".to_string(),
            "wider" => "0.05em".to_string(),
            "widest" => "0.1em".to_string(),
            _ => rest.to_string(),
        };
        return Some(("letter-spacing".to_string(), value));
    }

    // Line height
    if let Some(rest) = class.strip_prefix("leading-") {
        let value = match rest {
            "none" => "1".to_string(),
            "tight" => "1.25".to_string(),
            "snug" => "1.375".to_string(),
            "normal" => "1.5".to_string(),
            "relaxed" => "1.625".to_string(),
            "loose" => "2".to_string(),
            "3" => ".75rem".to_string(),
            "4" => "1rem".to_string(),
            "5" => "1.25rem".to_string(),
            "6" => "1.5rem".to_string(),
            "7" => "1.75rem".to_string(),
            "8" => "2rem".to_string(),
            "9" => "2.25rem".to_string(),
            "10" => "2.5rem".to_string(),
            _ => rest.to_string(),
        };
        return Some(("line-height".to_string(), value));
    }

    // List style type
    if let Some(rest) = class.strip_prefix("list-") {
        match rest {
            "inside" => return Some(("list-style-position".to_string(), "inside".to_string())),
            "outside" => return Some(("list-style-position".to_string(), "outside".to_string())),
            "none" => return Some(("list-style-type".to_string(), "none".to_string())),
            "disc" => return Some(("list-style-type".to_string(), "disc".to_string())),
            "decimal" => return Some(("list-style-type".to_string(), "decimal".to_string())),
            _ => {}
        }
    }

    // Vertical align
    if let Some(rest) = class.strip_prefix("align-") {
        return Some(("vertical-align".to_string(), rest.to_string()));
    }

    // Content
    if let Some(rest) = class.strip_prefix("content-") {
        if rest == "none" {
            return Some(("content".to_string(), "none".to_string()));
        }
    }

    None
}

/// Parse background utilities
fn parse_background_utility(class: &str) -> Option<(String, String)> {
    // Background color
    if let Some(rest) = class.strip_prefix("bg-") {
        // Check if it's a color
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("background-color".to_string(), color.to_string()));
        }
        // Background attachment
        match rest {
            "fixed" => return Some(("background-attachment".to_string(), "fixed".to_string())),
            "local" => return Some(("background-attachment".to_string(), "local".to_string())),
            "scroll" => return Some(("background-attachment".to_string(), "scroll".to_string())),
            // Background clip
            "clip-border" => {
                return Some(("background-clip".to_string(), "border-box".to_string()));
            }
            "clip-padding" => {
                return Some(("background-clip".to_string(), "padding-box".to_string()));
            }
            "clip-content" => {
                return Some(("background-clip".to_string(), "content-box".to_string()));
            }
            "clip-text" => return Some(("background-clip".to_string(), "text".to_string())),
            // Background origin
            "origin-border" => {
                return Some(("background-origin".to_string(), "border-box".to_string()));
            }
            "origin-padding" => {
                return Some(("background-origin".to_string(), "padding-box".to_string()));
            }
            "origin-content" => {
                return Some(("background-origin".to_string(), "content-box".to_string()));
            }
            // Background position
            "bottom" => return Some(("background-position".to_string(), "bottom".to_string())),
            "center" => return Some(("background-position".to_string(), "center".to_string())),
            "left" => return Some(("background-position".to_string(), "left".to_string())),
            "left-bottom" => {
                return Some(("background-position".to_string(), "left bottom".to_string()));
            }
            "left-top" => return Some(("background-position".to_string(), "left top".to_string())),
            "right" => return Some(("background-position".to_string(), "right".to_string())),
            "right-bottom" => {
                return Some((
                    "background-position".to_string(),
                    "right bottom".to_string(),
                ));
            }
            "right-top" => {
                return Some(("background-position".to_string(), "right top".to_string()));
            }
            "top" => return Some(("background-position".to_string(), "top".to_string())),
            // Background repeat
            "repeat" => return Some(("background-repeat".to_string(), "repeat".to_string())),
            "no-repeat" => return Some(("background-repeat".to_string(), "no-repeat".to_string())),
            "repeat-x" => return Some(("background-repeat".to_string(), "repeat-x".to_string())),
            "repeat-y" => return Some(("background-repeat".to_string(), "repeat-y".to_string())),
            "repeat-round" => return Some(("background-repeat".to_string(), "round".to_string())),
            "repeat-space" => return Some(("background-repeat".to_string(), "space".to_string())),
            // Background size
            "auto" => return Some(("background-size".to_string(), "auto".to_string())),
            "cover" => return Some(("background-size".to_string(), "cover".to_string())),
            "contain" => return Some(("background-size".to_string(), "contain".to_string())),
            // Gradients
            "none" => return Some(("background-image".to_string(), "none".to_string())),
            _ => {}
        }

        // Gradient directions
        if let Some(dir) = rest.strip_prefix("gradient-to-") {
            let direction = match dir {
                "t" => "to top".to_string(),
                "tr" => "to top right".to_string(),
                "r" => "to right".to_string(),
                "br" => "to bottom right".to_string(),
                "b" => "to bottom".to_string(),
                "bl" => "to bottom left".to_string(),
                "l" => "to left".to_string(),
                "tl" => "to top left".to_string(),
                _ => return None,
            };
            return Some((
                "background-image".to_string(),
                format!("linear-gradient({}, var(--tw-gradient-stops))", direction),
            ));
        }
    }

    // Gradient color stops
    if let Some(rest) = class.strip_prefix("from-") {
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("--tw-gradient-from".to_string(), color.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("via-") {
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("--tw-gradient-via".to_string(), color.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("to-") {
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("--tw-gradient-to".to_string(), color.to_string()));
        }
    }

    None
}

/// Parse border utilities
fn parse_border_utility(class: &str) -> Option<(String, String)> {
    // Border radius
    if let Some(rest) = class.strip_prefix("rounded-") {
        // Specific corners
        if let Some(corner) = rest.strip_prefix("t-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-top-left-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("r-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-top-right-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("b-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-bottom-right-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("l-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-bottom-left-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("tl-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-top-left-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("tr-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-top-right-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("br-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-bottom-right-radius".to_string(), value.to_string()));
            }
        }
        if let Some(corner) = rest.strip_prefix("bl-") {
            if let Some(&value) = BORDER_RADIUS_SCALE.get(corner) {
                return Some(("border-bottom-left-radius".to_string(), value.to_string()));
            }
        }
        if let Some(&value) = BORDER_RADIUS_SCALE.get(rest) {
            return Some(("border-radius".to_string(), value.to_string()));
        }
    }

    // Border without prefix
    if class == "rounded" {
        return Some(("border-radius".to_string(), "0.25rem".to_string()));
    }
    if class == "rounded-none" {
        return Some(("border-radius".to_string(), "0px".to_string()));
    }
    if class == "rounded-sm" {
        return Some(("border-radius".to_string(), "0.125rem".to_string()));
    }
    if class == "rounded-md" {
        return Some(("border-radius".to_string(), "0.375rem".to_string()));
    }
    if class == "rounded-lg" {
        return Some(("border-radius".to_string(), "0.5rem".to_string()));
    }
    if class == "rounded-xl" {
        return Some(("border-radius".to_string(), "0.75rem".to_string()));
    }
    if class == "rounded-2xl" {
        return Some(("border-radius".to_string(), "1rem".to_string()));
    }
    if class == "rounded-3xl" {
        return Some(("border-radius".to_string(), "1.5rem".to_string()));
    }
    if class == "rounded-full" {
        return Some(("border-radius".to_string(), "9999px".to_string()));
    }

    // Border width
    if let Some(rest) = class.strip_prefix("border-") {
        // Border color
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("border-color".to_string(), color.to_string()));
        }

        // Border width per side
        if let Some(width) = rest.strip_prefix("t-") {
            if let Some(&value) = BORDER_WIDTH_SCALE.get(width) {
                return Some(("border-top-width".to_string(), value.to_string()));
            }
        }
        if let Some(width) = rest.strip_prefix("r-") {
            if let Some(&value) = BORDER_WIDTH_SCALE.get(width) {
                return Some(("border-right-width".to_string(), value.to_string()));
            }
        }
        if let Some(width) = rest.strip_prefix("b-") {
            if let Some(&value) = BORDER_WIDTH_SCALE.get(width) {
                return Some(("border-bottom-width".to_string(), value.to_string()));
            }
        }
        if let Some(width) = rest.strip_prefix("l-") {
            if let Some(&value) = BORDER_WIDTH_SCALE.get(width) {
                return Some(("border-left-width".to_string(), value.to_string()));
            }
        }
        if let Some(width) = rest.strip_prefix("x-") {
            if let Some(&value) = BORDER_WIDTH_SCALE.get(width) {
                return Some(("border-inline-width".to_string(), value.to_string()));
            }
        }
        if let Some(width) = rest.strip_prefix("y-") {
            if let Some(&value) = BORDER_WIDTH_SCALE.get(width) {
                return Some(("border-block-width".to_string(), value.to_string()));
            }
        }

        // Border width
        if let Some(&value) = BORDER_WIDTH_SCALE.get(rest) {
            return Some(("border-width".to_string(), value.to_string()));
        }

        // Border style
        match rest {
            "solid" => return Some(("border-style".to_string(), "solid".to_string())),
            "dashed" => return Some(("border-style".to_string(), "dashed".to_string())),
            "dotted" => return Some(("border-style".to_string(), "dotted".to_string())),
            "double" => return Some(("border-style".to_string(), "double".to_string())),
            "hidden" => return Some(("border-style".to_string(), "hidden".to_string())),
            "none" => return Some(("border-style".to_string(), "none".to_string())),
            _ => {}
        }

        // Border collapse (for tables)
        if rest == "collapse" {
            return Some(("border-collapse".to_string(), "collapse".to_string()));
        }
        if rest == "separate" {
            return Some(("border-collapse".to_string(), "separate".to_string()));
        }
    }

    // Border without suffix (default 1px)
    if class == "border" {
        return Some(("border-width".to_string(), "1px".to_string()));
    }
    if class == "border-0" {
        return Some(("border-width".to_string(), "0px".to_string()));
    }
    if class == "border-2" {
        return Some(("border-width".to_string(), "2px".to_string()));
    }
    if class == "border-4" {
        return Some(("border-width".to_string(), "4px".to_string()));
    }
    if class == "border-8" {
        return Some(("border-width".to_string(), "8px".to_string()));
    }

    // Outline
    if let Some(rest) = class.strip_prefix("outline-") {
        match rest {
            "none" => {
                return Some(("outline".to_string(), "2px solid transparent".to_string()));
            }
            "0" => return Some(("outline-width".to_string(), "0px".to_string())),
            "1" => return Some(("outline-width".to_string(), "1px".to_string())),
            "2" => return Some(("outline-width".to_string(), "2px".to_string())),
            "4" => return Some(("outline-width".to_string(), "4px".to_string())),
            "8" => return Some(("outline-width".to_string(), "8px".to_string())),
            "dashed" => return Some(("outline-style".to_string(), "dashed".to_string())),
            "dotted" => return Some(("outline-style".to_string(), "dotted".to_string())),
            "double" => return Some(("outline-style".to_string(), "double".to_string())),
            _ => {
                if let Some(&color) = TAILWIND_COLORS.get(rest) {
                    return Some(("outline-color".to_string(), color.to_string()));
                }
            }
        }
    }
    if class == "outline" {
        return Some(("outline-style".to_string(), "solid".to_string()));
    }

    // Ring
    if let Some(rest) = class.strip_prefix("ring-") {
        match rest {
            "0" => {
                return Some((
                    "--tw-ring-offset-shadow".to_string(),
                    "0 0 #0000".to_string(),
                ));
            }
            "1" => {
                return Some((
                    "box-shadow".to_string(),
                    "0 0 0 1px var(--tw-ring-color)".to_string(),
                ));
            }
            "2" => {
                return Some((
                    "box-shadow".to_string(),
                    "0 0 0 2px var(--tw-ring-color)".to_string(),
                ));
            }
            "4" => {
                return Some((
                    "box-shadow".to_string(),
                    "0 0 0 4px var(--tw-ring-color)".to_string(),
                ));
            }
            "8" => {
                return Some((
                    "box-shadow".to_string(),
                    "0 0 0 8px var(--tw-ring-color)".to_string(),
                ));
            }
            "inset" => return Some(("--tw-ring-inset".to_string(), "inset".to_string())),
            _ => {
                if let Some(&color) = TAILWIND_COLORS.get(rest) {
                    return Some(("--tw-ring-color".to_string(), color.to_string()));
                }
            }
        }
    }
    if class == "ring" {
        return Some((
            "box-shadow".to_string(),
            "0 0 0 3px var(--tw-ring-color)".to_string(),
        ));
    }

    // Divide
    if let Some(rest) = class.strip_prefix("divide-") {
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("--tw-divide-color".to_string(), color.to_string()));
        }
        match rest {
            "x" => return Some(("--tw-divide-x-reverse".to_string(), "0".to_string())),
            "x-0" => return Some(("border-inline-width".to_string(), "0px".to_string())),
            "x-2" => return Some(("border-inline-width".to_string(), "2px".to_string())),
            "x-4" => return Some(("border-inline-width".to_string(), "4px".to_string())),
            "x-8" => return Some(("border-inline-width".to_string(), "8px".to_string())),
            "x-reverse" => return Some(("--tw-divide-x-reverse".to_string(), "1".to_string())),
            "y" => return Some(("--tw-divide-y-reverse".to_string(), "0".to_string())),
            "y-0" => return Some(("border-block-width".to_string(), "0px".to_string())),
            "y-2" => return Some(("border-block-width".to_string(), "2px".to_string())),
            "y-4" => return Some(("border-block-width".to_string(), "4px".to_string())),
            "y-8" => return Some(("border-block-width".to_string(), "8px".to_string())),
            "y-reverse" => return Some(("--tw-divide-y-reverse".to_string(), "1".to_string())),
            "solid" => return Some(("border-style".to_string(), "solid".to_string())),
            "dashed" => return Some(("border-style".to_string(), "dashed".to_string())),
            "dotted" => return Some(("border-style".to_string(), "dotted".to_string())),
            "double" => return Some(("border-style".to_string(), "double".to_string())),
            "none" => return Some(("border-style".to_string(), "none".to_string())),
            _ => {}
        }
    }

    None
}

/// Parse effects utilities (shadow, opacity, mix-blend, etc.)
fn parse_effects_utility(class: &str) -> Option<(String, String)> {
    // Box shadow
    if let Some(rest) = class.strip_prefix("shadow-") {
        if let Some(&value) = BOX_SHADOW_SCALE.get(rest) {
            return Some(("box-shadow".to_string(), value.to_string()));
        }
        // Shadow color
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("--tw-shadow-color".to_string(), color.to_string()));
        }
    }
    if class == "shadow" {
        return Some((
            "box-shadow".to_string(),
            "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)".to_string(),
        ));
    }

    // Opacity
    if let Some(rest) = class.strip_prefix("opacity-") {
        if let Some(&value) = OPACITY_SCALE.get(rest) {
            return Some(("opacity".to_string(), value.to_string()));
        }
    }

    // Mix blend mode
    if let Some(rest) = class.strip_prefix("mix-blend-") {
        return Some(("mix-blend-mode".to_string(), rest.to_string()));
    }

    // Background blend mode
    if let Some(rest) = class.strip_prefix("bg-blend-") {
        return Some(("background-blend-mode".to_string(), rest.to_string()));
    }

    None
}

/// Parse filter utilities (blur, brightness, contrast, etc.)
fn parse_filter_utility(class: &str) -> Option<(String, String)> {
    // Blur
    if let Some(rest) = class.strip_prefix("blur-") {
        let value = match rest {
            "none" => "0".to_string(),
            "sm" => "4px".to_string(),
            "md" => "12px".to_string(),
            "lg" => "16px".to_string(),
            "xl" => "24px".to_string(),
            "2xl" => "40px".to_string(),
            "3xl" => "64px".to_string(),
            _ => return None,
        };
        return Some(("filter".to_string(), format!("blur({})", value)));
    }
    if class == "blur" {
        return Some(("filter".to_string(), "blur(8px)".to_string()));
    }

    // Brightness
    if let Some(rest) = class.strip_prefix("brightness-") {
        let value = match rest {
            "0" => "0".to_string(),
            "50" => ".5".to_string(),
            "75" => ".75".to_string(),
            "90" => ".9".to_string(),
            "95" => ".95".to_string(),
            "100" => "1".to_string(),
            "105" => "1.05".to_string(),
            "110" => "1.1".to_string(),
            "125" => "1.25".to_string(),
            "150" => "1.5".to_string(),
            "200" => "2".to_string(),
            _ => return None,
        };
        return Some(("filter".to_string(), format!("brightness({})", value)));
    }

    // Contrast
    if let Some(rest) = class.strip_prefix("contrast-") {
        let value = match rest {
            "0" => "0".to_string(),
            "50" => ".5".to_string(),
            "75" => ".75".to_string(),
            "100" => "1".to_string(),
            "125" => "1.25".to_string(),
            "150" => "1.5".to_string(),
            "200" => "2".to_string(),
            _ => return None,
        };
        return Some(("filter".to_string(), format!("contrast({})", value)));
    }

    // Drop shadow
    if let Some(rest) = class.strip_prefix("drop-shadow-") {
        let value = match rest {
            "sm" => "drop-shadow(0 1px 1px rgb(0 0 0 / 0.05))".to_string(),
            "md" => "drop-shadow(0 4px 3px rgb(0 0 0 / 0.07)) drop-shadow(0 2px 2px rgb(0 0 0 / 0.06))".to_string(),
            "lg" => "drop-shadow(0 10px 8px rgb(0 0 0 / 0.04)) drop-shadow(0 4px 3px rgb(0 0 0 / 0.1))".to_string(),
            "xl" => "drop-shadow(0 20px 13px rgb(0 0 0 / 0.03)) drop-shadow(0 8px 5px rgb(0 0 0 / 0.08))".to_string(),
            "2xl" => "drop-shadow(0 25px 25px rgb(0 0 0 / 0.15))".to_string(),
            "none" => "drop-shadow(0 0 #0000)".to_string(),
            _ => return None,
        };
        return Some(("filter".to_string(), value));
    }
    if class == "drop-shadow" {
        return Some((
            "filter".to_string(),
            "drop-shadow(0 1px 2px rgb(0 0 0 / 0.1)) drop-shadow(0 1px 1px rgb(0 0 0 / 0.06))"
                .to_string(),
        ));
    }

    // Grayscale
    if class == "grayscale" {
        return Some(("filter".to_string(), "grayscale(100%)".to_string()));
    }
    if class == "grayscale-0" {
        return Some(("filter".to_string(), "grayscale(0)".to_string()));
    }

    // Hue rotate
    if let Some(rest) = class.strip_prefix("hue-rotate-") {
        let value = match rest {
            "0" => "0deg".to_string(),
            "15" => "15deg".to_string(),
            "30" => "30deg".to_string(),
            "60" => "60deg".to_string(),
            "90" => "90deg".to_string(),
            "180" => "180deg".to_string(),
            _ => return None,
        };
        return Some(("filter".to_string(), format!("hue-rotate({})", value)));
    }

    // Invert
    if class == "invert" {
        return Some(("filter".to_string(), "invert(100%)".to_string()));
    }
    if class == "invert-0" {
        return Some(("filter".to_string(), "invert(0)".to_string()));
    }

    // Saturate
    if let Some(rest) = class.strip_prefix("saturate-") {
        let value = match rest {
            "0" => "0".to_string(),
            "50" => ".5".to_string(),
            "100" => "1".to_string(),
            "150" => "1.5".to_string(),
            "200" => "2".to_string(),
            _ => return None,
        };
        return Some(("filter".to_string(), format!("saturate({})", value)));
    }

    // Sepia
    if class == "sepia" {
        return Some(("filter".to_string(), "sepia(100%)".to_string()));
    }
    if class == "sepia-0" {
        return Some(("filter".to_string(), "sepia(0)".to_string()));
    }

    // Backdrop filters
    if let Some(rest) = class.strip_prefix("backdrop-blur-") {
        let value = match rest {
            "none" => "0".to_string(),
            "sm" => "4px".to_string(),
            "md" => "12px".to_string(),
            "lg" => "16px".to_string(),
            "xl" => "24px".to_string(),
            "2xl" => "40px".to_string(),
            "3xl" => "64px".to_string(),
            _ => return None,
        };
        return Some(("backdrop-filter".to_string(), format!("blur({})", value)));
    }
    if class == "backdrop-blur" {
        return Some(("backdrop-filter".to_string(), "blur(8px)".to_string()));
    }

    if let Some(rest) = class.strip_prefix("backdrop-brightness-") {
        let value = match rest {
            "0" => "0".to_string(),
            "50" => ".5".to_string(),
            "75" => ".75".to_string(),
            "90" => ".9".to_string(),
            "95" => ".95".to_string(),
            "100" => "1".to_string(),
            "105" => "1.05".to_string(),
            "110" => "1.1".to_string(),
            "125" => "1.25".to_string(),
            "150" => "1.5".to_string(),
            "200" => "2".to_string(),
            _ => return None,
        };
        return Some((
            "backdrop-filter".to_string(),
            format!("brightness({})", value),
        ));
    }

    if let Some(rest) = class.strip_prefix("backdrop-contrast-") {
        let value = match rest {
            "0" => "0".to_string(),
            "50" => ".5".to_string(),
            "75" => ".75".to_string(),
            "100" => "1".to_string(),
            "125" => "1.25".to_string(),
            "150" => "1.5".to_string(),
            "200" => "2".to_string(),
            _ => return None,
        };
        return Some((
            "backdrop-filter".to_string(),
            format!("contrast({})", value),
        ));
    }

    if class == "backdrop-grayscale" {
        return Some(("backdrop-filter".to_string(), "grayscale(100%)".to_string()));
    }
    if class == "backdrop-grayscale-0" {
        return Some(("backdrop-filter".to_string(), "grayscale(0)".to_string()));
    }

    if class == "backdrop-invert" {
        return Some(("backdrop-filter".to_string(), "invert(100%)".to_string()));
    }
    if class == "backdrop-invert-0" {
        return Some(("backdrop-filter".to_string(), "invert(0)".to_string()));
    }

    if let Some(rest) = class.strip_prefix("backdrop-opacity-") {
        if let Some(&value) = OPACITY_SCALE.get(rest) {
            return Some(("backdrop-filter".to_string(), format!("opacity({})", value)));
        }
    }

    if let Some(rest) = class.strip_prefix("backdrop-saturate-") {
        let value = match rest {
            "0" => "0".to_string(),
            "50" => ".5".to_string(),
            "100" => "1".to_string(),
            "150" => "1.5".to_string(),
            "200" => "2".to_string(),
            _ => return None,
        };
        return Some((
            "backdrop-filter".to_string(),
            format!("saturate({})", value),
        ));
    }

    if class == "backdrop-sepia" {
        return Some(("backdrop-filter".to_string(), "sepia(100%)".to_string()));
    }
    if class == "backdrop-sepia-0" {
        return Some(("backdrop-filter".to_string(), "sepia(0)".to_string()));
    }

    None
}

/// Parse transition and animation utilities
fn parse_transition_utility(class: &str) -> Option<(String, String)> {
    // Transition
    match class {
        "transition-none" => return Some(("transition-property".to_string(), "none".to_string())),
        "transition-all" => return Some(("transition-property".to_string(), "all".to_string())),
        "transition" => return Some(("transition-property".to_string(), "color, background-color, border-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform, filter, backdrop-filter".to_string())),
        "transition-colors" => return Some(("transition-property".to_string(), "color, background-color, border-color, text-decoration-color, fill, stroke".to_string())),
        "transition-opacity" => return Some(("transition-property".to_string(), "opacity".to_string())),
        "transition-shadow" => return Some(("transition-property".to_string(), "box-shadow".to_string())),
        "transition-transform" => return Some(("transition-property".to_string(), "transform".to_string())),
        _ => {}
    }

    // Duration
    if let Some(rest) = class.strip_prefix("duration-") {
        if let Some(&value) = DURATION_SCALE.get(rest) {
            return Some(("transition-duration".to_string(), value.to_string()));
        }
    }

    // Ease (timing function)
    if let Some(rest) = class.strip_prefix("ease-") {
        if let Some(&value) = EASE_SCALE.get(rest) {
            return Some(("transition-timing-function".to_string(), value.to_string()));
        }
    }

    // Delay
    if let Some(rest) = class.strip_prefix("delay-") {
        if let Some(&value) = DURATION_SCALE.get(rest) {
            return Some(("transition-delay".to_string(), value.to_string()));
        }
    }

    // Animation
    match class {
        "animate-none" => return Some(("animation".to_string(), "none".to_string())),
        "animate-spin" => {
            return Some((
                "animation".to_string(),
                "spin 1s linear infinite".to_string(),
            ));
        }
        "animate-ping" => {
            return Some((
                "animation".to_string(),
                "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite".to_string(),
            ));
        }
        "animate-pulse" => {
            return Some((
                "animation".to_string(),
                "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite".to_string(),
            ));
        }
        "animate-bounce" => {
            return Some(("animation".to_string(), "bounce 1s infinite".to_string()));
        }
        _ => {}
    }

    None
}

/// Parse transform utilities (scale, rotate, translate, skew)
fn parse_transform_utility(class: &str, is_negative: bool) -> Option<(String, String)> {
    // Scale
    if let Some(rest) = class.strip_prefix("scale-x-") {
        let value = parse_scale_value(rest)?;
        return Some(("transform".to_string(), format!("scaleX({})", value)));
    }
    if let Some(rest) = class.strip_prefix("scale-y-") {
        let value = parse_scale_value(rest)?;
        return Some(("transform".to_string(), format!("scaleY({})", value)));
    }
    if let Some(rest) = class.strip_prefix("scale-") {
        let value = parse_scale_value(rest)?;
        return Some(("transform".to_string(), format!("scale({})", value)));
    }

    // Rotate
    if let Some(rest) = class.strip_prefix("rotate-") {
        let value = match rest {
            "0" => "0deg".to_string(),
            "1" => "1deg".to_string(),
            "2" => "2deg".to_string(),
            "3" => "3deg".to_string(),
            "6" => "6deg".to_string(),
            "12" => "12deg".to_string(),
            "45" => "45deg".to_string(),
            "90" => "90deg".to_string(),
            "180" => "180deg".to_string(),
            _ => return None,
        };
        let neg_prefix = if is_negative { "-" } else { "" };
        return Some((
            "transform".to_string(),
            format!("rotate({}{})", neg_prefix, value),
        ));
    }

    // Translate
    if let Some(rest) = class.strip_prefix("translate-x-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            let neg_prefix = if is_negative { "-" } else { "" };
            return Some((
                "transform".to_string(),
                format!("translateX({}{})", neg_prefix, value),
            ));
        }
    }
    if let Some(rest) = class.strip_prefix("translate-y-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            let neg_prefix = if is_negative { "-" } else { "" };
            return Some((
                "transform".to_string(),
                format!("translateY({}{})", neg_prefix, value),
            ));
        }
    }

    // Skew
    if let Some(rest) = class.strip_prefix("skew-x-") {
        let value = match rest {
            "0" => "0deg".to_string(),
            "1" => "1deg".to_string(),
            "2" => "2deg".to_string(),
            "3" => "3deg".to_string(),
            "6" => "6deg".to_string(),
            "12" => "12deg".to_string(),
            _ => return None,
        };
        let neg_prefix = if is_negative { "-" } else { "" };
        return Some((
            "transform".to_string(),
            format!("skewX({}{})", neg_prefix, value),
        ));
    }
    if let Some(rest) = class.strip_prefix("skew-y-") {
        let value = match rest {
            "0" => "0deg".to_string(),
            "1" => "1deg".to_string(),
            "2" => "2deg".to_string(),
            "3" => "3deg".to_string(),
            "6" => "6deg".to_string(),
            "12" => "12deg".to_string(),
            _ => return None,
        };
        let neg_prefix = if is_negative { "-" } else { "" };
        return Some((
            "transform".to_string(),
            format!("skewY({}{})", neg_prefix, value),
        ));
    }

    // Transform origin
    if let Some(rest) = class.strip_prefix("origin-") {
        let value = match rest {
            "center" => "center".to_string(),
            "top" => "top".to_string(),
            "top-right" => "top right".to_string(),
            "right" => "right".to_string(),
            "bottom-right" => "bottom right".to_string(),
            "bottom" => "bottom".to_string(),
            "bottom-left" => "bottom left".to_string(),
            "left" => "left".to_string(),
            "top-left" => "top left".to_string(),
            _ => return None,
        };
        return Some(("transform-origin".to_string(), value));
    }

    None
}

/// Parse scale value (50 -> 0.5, 100 -> 1, 150 -> 1.5)
fn parse_scale_value(s: &str) -> Option<f64> {
    let n: u32 = s.parse().ok()?;
    Some(n as f64 / 100.0)
}

/// Parse interactivity utilities (cursor, pointer-events, resize, etc.)
fn parse_interactivity_utility(class: &str) -> Option<(String, String)> {
    // Accent color
    if let Some(rest) = class.strip_prefix("accent-") {
        if rest == "auto" {
            return Some(("accent-color".to_string(), "auto".to_string()));
        }
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("accent-color".to_string(), color.to_string()));
        }
    }

    // Appearance
    match class {
        "appearance-none" => return Some(("appearance".to_string(), "none".to_string())),
        "appearance-auto" => return Some(("appearance".to_string(), "auto".to_string())),
        _ => {}
    }

    // Cursor
    if let Some(rest) = class.strip_prefix("cursor-") {
        return Some(("cursor".to_string(), rest.to_string()));
    }

    // Caret color
    if let Some(rest) = class.strip_prefix("caret-") {
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("caret-color".to_string(), color.to_string()));
        }
    }

    // Pointer events
    if let Some(rest) = class.strip_prefix("pointer-events-") {
        return Some(("pointer-events".to_string(), rest.to_string()));
    }

    // Resize
    match class {
        "resize-none" => return Some(("resize".to_string(), "none".to_string())),
        "resize-y" => return Some(("resize".to_string(), "vertical".to_string())),
        "resize-x" => return Some(("resize".to_string(), "horizontal".to_string())),
        "resize" => return Some(("resize".to_string(), "both".to_string())),
        _ => {}
    }

    // Scroll behavior
    if let Some(rest) = class.strip_prefix("scroll-") {
        match rest {
            "auto" => return Some(("scroll-behavior".to_string(), "auto".to_string())),
            "smooth" => return Some(("scroll-behavior".to_string(), "smooth".to_string())),
            _ => {}
        }
    }

    // Scroll margin/padding
    if let Some(rest) = class.strip_prefix("scroll-m-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("scroll-margin".to_string(), value.to_string()));
        }
    }
    if let Some(rest) = class.strip_prefix("scroll-p-") {
        if let Some(&value) = SPACING_SCALE.get(rest) {
            return Some(("scroll-padding".to_string(), value.to_string()));
        }
    }

    // Scroll snap
    if let Some(rest) = class.strip_prefix("snap-") {
        match rest {
            "start" => return Some(("scroll-snap-align".to_string(), "start".to_string())),
            "end" => return Some(("scroll-snap-align".to_string(), "end".to_string())),
            "center" => return Some(("scroll-snap-align".to_string(), "center".to_string())),
            "align-none" => return Some(("scroll-snap-align".to_string(), "none".to_string())),
            "none" => return Some(("scroll-snap-type".to_string(), "none".to_string())),
            "x" => {
                return Some((
                    "scroll-snap-type".to_string(),
                    "x var(--tw-scroll-snap-strictness)".to_string(),
                ));
            }
            "y" => {
                return Some((
                    "scroll-snap-type".to_string(),
                    "y var(--tw-scroll-snap-strictness)".to_string(),
                ));
            }
            "both" => {
                return Some((
                    "scroll-snap-type".to_string(),
                    "both var(--tw-scroll-snap-strictness)".to_string(),
                ));
            }
            "mandatory" => {
                return Some((
                    "--tw-scroll-snap-strictness".to_string(),
                    "mandatory".to_string(),
                ));
            }
            "proximity" => {
                return Some((
                    "--tw-scroll-snap-strictness".to_string(),
                    "proximity".to_string(),
                ));
            }
            "normal" => return Some(("scroll-snap-stop".to_string(), "normal".to_string())),
            "always" => return Some(("scroll-snap-stop".to_string(), "always".to_string())),
            _ => {}
        }
    }

    // Touch action
    if let Some(rest) = class.strip_prefix("touch-") {
        return Some(("touch-action".to_string(), rest.to_string()));
    }

    // User select
    if let Some(rest) = class.strip_prefix("select-") {
        return Some(("user-select".to_string(), rest.to_string()));
    }

    // Will change
    if let Some(rest) = class.strip_prefix("will-change-") {
        let value = match rest {
            "auto" => "auto".to_string(),
            "scroll" => "scroll-position".to_string(),
            "contents" => "contents".to_string(),
            "transform" => "transform".to_string(),
            _ => return None,
        };
        return Some(("will-change".to_string(), value));
    }

    None
}

/// Parse SVG utilities (fill, stroke)
fn parse_svg_utility(class: &str) -> Option<(String, String)> {
    // Fill
    if let Some(rest) = class.strip_prefix("fill-") {
        if rest == "none" {
            return Some(("fill".to_string(), "none".to_string()));
        }
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("fill".to_string(), color.to_string()));
        }
    }

    // Stroke
    if let Some(rest) = class.strip_prefix("stroke-") {
        if rest == "none" {
            return Some(("stroke".to_string(), "none".to_string()));
        }
        // Stroke width
        match rest {
            "0" => return Some(("stroke-width".to_string(), "0".to_string())),
            "1" => return Some(("stroke-width".to_string(), "1".to_string())),
            "2" => return Some(("stroke-width".to_string(), "2".to_string())),
            _ => {}
        }
        // Stroke color
        if let Some(&color) = TAILWIND_COLORS.get(rest) {
            return Some(("stroke".to_string(), color.to_string()));
        }
    }

    None
}

/// Parse accessibility utilities (screen reader only)
fn parse_accessibility_utility(class: &str) -> Option<(String, String)> {
    match class {
        "sr-only" => {
            // This utility requires multiple CSS properties
            // We'll return the most important one
            Some(("position".to_string(), "absolute".to_string()))
        }
        "not-sr-only" => Some(("position".to_string(), "static".to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use css::class_map::reset_class_map;
    use css::file_map::reset_file_map;
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serial_test::serial;
    use std::collections::BTreeSet;

    // Helper to sort styles for consistent snapshots
    fn sort_styles(styles: Vec<ExtractStyleValue>) -> BTreeSet<ExtractStyleValue> {
        styles.into_iter().collect()
    }

    #[test]
    fn test_has_tailwind_classes_basic() {
        assert!(has_tailwind_classes("bg-red-500 text-white"));
        assert!(has_tailwind_classes("p-4 m-2"));
        assert!(has_tailwind_classes("flex items-center"));
        assert!(!has_tailwind_classes("my-custom-class"));
        assert!(!has_tailwind_classes(""));
    }

    #[test]
    fn test_has_tailwind_classes_with_responsive() {
        assert!(has_tailwind_classes("sm:bg-blue-500"));
        assert!(has_tailwind_classes("md:flex lg:hidden"));
        assert!(has_tailwind_classes("hover:bg-red-500"));
    }

    #[test]
    fn test_has_tailwind_classes_with_arbitrary() {
        assert!(has_tailwind_classes("w-[100px]"));
        assert!(has_tailwind_classes("text-[#ff0000]"));
        assert!(has_tailwind_classes("p-[calc(100%-20px)]"));
    }

    #[rstest]
    #[case("bg-red-500", "background-color", "#ef4444")]
    #[case("bg-blue-500", "background-color", "#3b82f6")]
    #[case("bg-black", "background-color", "#000")]
    #[case("bg-white", "background-color", "#fff")]
    #[case("bg-transparent", "background-color", "transparent")]
    #[case("text-red-500", "color", "#ef4444")]
    #[case("text-white", "color", "#fff")]
    fn test_parse_color_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("p-4", "padding", "1rem")]
    #[case("p-0", "padding", "0px")]
    #[case("p-px", "padding", "1px")]
    #[case("p-0.5", "padding", "0.125rem")]
    #[case("px-4", "padding-inline", "1rem")]
    #[case("py-2", "padding-block", "0.5rem")]
    #[case("pt-4", "padding-top", "1rem")]
    #[case("pr-4", "padding-right", "1rem")]
    #[case("pb-4", "padding-bottom", "1rem")]
    #[case("pl-4", "padding-left", "1rem")]
    #[case("m-4", "margin", "1rem")]
    #[case("mx-auto", "margin-inline", "auto")]
    #[case("my-4", "margin-block", "1rem")]
    #[case("mt-4", "margin-top", "1rem")]
    fn test_parse_spacing_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("w-full", "width", "100%")]
    #[case("w-screen", "width", "100vw")]
    #[case("w-auto", "width", "auto")]
    #[case("w-1/2", "width", "50%")]
    #[case("w-4", "width", "1rem")]
    #[case("h-full", "height", "100%")]
    #[case("h-screen", "height", "100vh")]
    #[case("min-w-0", "min-width", "0px")]
    #[case("min-w-full", "min-width", "100%")]
    #[case("max-w-sm", "max-width", "24rem")]
    #[case("max-w-xl", "max-width", "36rem")]
    fn test_parse_sizing_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("flex", "display", "flex")]
    #[case("inline-flex", "display", "inline-flex")]
    #[case("grid", "display", "grid")]
    #[case("block", "display", "block")]
    #[case("hidden", "display", "none")]
    #[case("absolute", "position", "absolute")]
    #[case("relative", "position", "relative")]
    #[case("fixed", "position", "fixed")]
    #[case("sticky", "position", "sticky")]
    fn test_parse_layout_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("flex-row", "flex-direction", "row")]
    #[case("flex-col", "flex-direction", "column")]
    #[case("flex-wrap", "flex-wrap", "wrap")]
    #[case("flex-1", "flex", "1 1 0%")]
    #[case("justify-center", "justify-content", "center")]
    #[case("justify-between", "justify-content", "space-between")]
    #[case("items-center", "align-items", "center")]
    #[case("items-start", "align-items", "flex-start")]
    #[case("gap-4", "gap", "1rem")]
    fn test_parse_flex_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("font-bold", "font-weight", "700")]
    #[case("font-normal", "font-weight", "400")]
    #[case("text-sm", "font-size", "0.875rem")]
    #[case("text-xl", "font-size", "1.25rem")]
    #[case("text-center", "text-align", "center")]
    #[case("italic", "font-style", "italic")]
    #[case("underline", "text-decoration-line", "underline")]
    #[case("uppercase", "text-transform", "uppercase")]
    fn test_parse_typography_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("rounded", "border-radius", "0.25rem")]
    #[case("rounded-none", "border-radius", "0px")]
    #[case("rounded-full", "border-radius", "9999px")]
    #[case("rounded-lg", "border-radius", "0.5rem")]
    #[case("border", "border-width", "1px")]
    #[case("border-2", "border-width", "2px")]
    #[case("border-red-500", "border-color", "#ef4444")]
    fn test_parse_border_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("opacity-50", "opacity", "0.5")]
    #[case("opacity-100", "opacity", "1")]
    #[case("opacity-0", "opacity", "0")]
    fn test_parse_opacity_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("z-10", "z-index", "10")]
    #[case("z-50", "z-index", "50")]
    #[case("z-auto", "z-index", "auto")]
    fn test_parse_z_index_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[test]
    fn test_parse_responsive_prefix() {
        let parsed = parse_single_class("sm:bg-red-500").expect("Should parse");
        assert_eq!(parsed.responsive, 1);
        assert_eq!(parsed.property, "background-color");
        assert_eq!(parsed.value, "#ef4444");

        let parsed = parse_single_class("md:p-4").expect("Should parse");
        assert_eq!(parsed.responsive, 2);
        assert_eq!(parsed.property, "padding");
        assert_eq!(parsed.value, "1rem");

        let parsed = parse_single_class("lg:flex").expect("Should parse");
        assert_eq!(parsed.responsive, 3);
        assert_eq!(parsed.property, "display");
        assert_eq!(parsed.value, "flex");

        let parsed = parse_single_class("xl:hidden").expect("Should parse");
        assert_eq!(parsed.responsive, 4);
        assert_eq!(parsed.property, "display");
        assert_eq!(parsed.value, "none");

        let parsed = parse_single_class("2xl:w-full").expect("Should parse");
        assert_eq!(parsed.responsive, 5);
        assert_eq!(parsed.property, "width");
        assert_eq!(parsed.value, "100%");
    }

    #[test]
    fn test_parse_variant_hover() {
        let parsed = parse_single_class("hover:bg-blue-500").expect("Should parse");
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::Hover);
        assert_eq!(parsed.property, "background-color");
        assert_eq!(parsed.value, "#3b82f6");
    }

    #[test]
    fn test_parse_variant_focus() {
        let parsed = parse_single_class("focus:outline-none").expect("Should parse");
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::Focus);
    }

    #[test]
    fn test_parse_variant_dark() {
        let parsed = parse_single_class("dark:bg-gray-800").expect("Should parse");
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::Dark);
        assert_eq!(parsed.property, "background-color");
        assert_eq!(parsed.value, "#1f2937");
    }

    #[test]
    fn test_parse_combined_responsive_variant() {
        let parsed = parse_single_class("sm:hover:bg-red-500").expect("Should parse");
        assert_eq!(parsed.responsive, 1);
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::Hover);
        assert_eq!(parsed.property, "background-color");
    }

    #[test]
    fn test_parse_arbitrary_width() {
        let parsed = parse_single_class("w-[100px]").expect("Should parse");
        assert_eq!(parsed.property, "width");
        assert_eq!(parsed.value, "100px");
    }

    #[test]
    fn test_parse_arbitrary_color() {
        let parsed = parse_single_class("bg-[#ff0000]").expect("Should parse");
        assert_eq!(parsed.property, "background-color");
        assert_eq!(parsed.value, "#ff0000");
    }

    #[test]
    fn test_parse_arbitrary_calc() {
        let parsed = parse_single_class("w-[calc(100%-20px)]").expect("Should parse");
        assert_eq!(parsed.property, "width");
        assert_eq!(parsed.value, "calc(100%-20px)");
    }

    #[test]
    fn test_parse_negative_margin() {
        let parsed = parse_single_class("-m-4").expect("Should parse");
        assert_eq!(parsed.property, "margin");
        assert_eq!(parsed.value, "1rem");
        assert!(parsed.negative);

        let static_style = parsed.to_static_style();
        assert_eq!(static_style.value(), "-1rem");
    }

    #[test]
    fn test_parse_negative_translate() {
        let parsed = parse_single_class("-translate-x-4").expect("Should parse");
        assert_eq!(parsed.property, "transform");
        assert_eq!(parsed.value, "translateX(-1rem)");
    }

    #[test]
    fn test_variant_to_selector() {
        assert_eq!(
            TailwindVariant::Hover.to_selector(),
            StyleSelector::Selector("&:hover".to_string())
        );
        assert_eq!(
            TailwindVariant::Focus.to_selector(),
            StyleSelector::Selector("&:focus".to_string())
        );
        assert_eq!(
            TailwindVariant::Dark.to_selector(),
            StyleSelector::Selector(":root[data-theme=dark] &".to_string())
        );
        assert_eq!(
            TailwindVariant::GroupHover.to_selector(),
            StyleSelector::Selector("*[role=group]:hover &".to_string())
        );
    }

    #[test]
    fn test_to_static_style() {
        let parsed = parse_single_class("bg-red-500").expect("Should parse");
        let static_style = parsed.to_static_style();

        assert_eq!(static_style.property(), "background-color");
        // ExtractStaticStyle::new() uses optimize_value() which uppercases hex colors
        assert_eq!(static_style.value(), "#EF4444");
        assert_eq!(static_style.level(), 0);
        assert!(static_style.selector().is_none());
    }

    #[test]
    fn test_to_static_style_with_responsive() {
        let parsed = parse_single_class("md:p-4").expect("Should parse");
        let static_style = parsed.to_static_style();

        assert_eq!(static_style.property(), "padding");
        assert_eq!(static_style.value(), "1rem");
        assert_eq!(static_style.level(), 2);
    }

    #[test]
    fn test_to_static_style_with_variant() {
        let parsed = parse_single_class("hover:bg-blue-500").expect("Should parse");
        let static_style = parsed.to_static_style();

        assert_eq!(static_style.property(), "background-color");
        assert!(static_style.selector().is_some());
        let selector = static_style.selector().unwrap();
        assert_eq!(selector.to_string(), "&:hover");
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_basic() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles("bg-red-500 p-4 flex", None);
        assert_eq!(styles.len(), 3);

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_responsive() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles("sm:bg-blue-500 md:p-8 lg:flex", None);
        assert_eq!(styles.len(), 3);

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_variants() {
        reset_class_map();
        reset_file_map();

        let styles =
            parse_tailwind_to_styles("hover:bg-blue-500 focus:outline-none dark:text-white", None);

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_complex() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles(
            "flex items-center justify-between p-4 bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-200",
            None,
        );

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    fn test_parse_grid_utilities() {
        let parsed = parse_single_class("grid-cols-3").expect("Should parse");
        assert_eq!(parsed.property, "grid-template-columns");
        assert_eq!(parsed.value, "repeat(3, minmax(0, 1fr))");

        let parsed = parse_single_class("col-span-2").expect("Should parse");
        assert_eq!(parsed.property, "grid-column");
        assert_eq!(parsed.value, "span 2 / span 2");

        let parsed = parse_single_class("row-span-3").expect("Should parse");
        assert_eq!(parsed.property, "grid-row");
        assert_eq!(parsed.value, "span 3 / span 3");
    }

    #[test]
    fn test_parse_transition_utilities() {
        let parsed = parse_single_class("transition").expect("Should parse");
        assert_eq!(parsed.property, "transition-property");

        let parsed = parse_single_class("duration-300").expect("Should parse");
        assert_eq!(parsed.property, "transition-duration");
        assert_eq!(parsed.value, "300ms");

        let parsed = parse_single_class("ease-in-out").expect("Should parse");
        assert_eq!(parsed.property, "transition-timing-function");
    }

    #[test]
    fn test_parse_transform_utilities() {
        let parsed = parse_single_class("scale-50").expect("Should parse");
        assert_eq!(parsed.property, "transform");
        assert_eq!(parsed.value, "scale(0.5)");

        let parsed = parse_single_class("rotate-45").expect("Should parse");
        assert_eq!(parsed.property, "transform");
        assert_eq!(parsed.value, "rotate(45deg)");

        let parsed = parse_single_class("translate-x-4").expect("Should parse");
        assert_eq!(parsed.property, "transform");
        assert_eq!(parsed.value, "translateX(1rem)");
    }

    #[test]
    fn test_parse_filter_utilities() {
        let parsed = parse_single_class("blur").expect("Should parse");
        assert_eq!(parsed.property, "filter");
        assert_eq!(parsed.value, "blur(8px)");

        let parsed = parse_single_class("blur-lg").expect("Should parse");
        assert_eq!(parsed.property, "filter");
        assert_eq!(parsed.value, "blur(16px)");

        let parsed = parse_single_class("grayscale").expect("Should parse");
        assert_eq!(parsed.property, "filter");
        assert_eq!(parsed.value, "grayscale(100%)");
    }

    #[test]
    fn test_parse_interactivity_utilities() {
        let parsed = parse_single_class("cursor-pointer").expect("Should parse");
        assert_eq!(parsed.property, "cursor");
        assert_eq!(parsed.value, "pointer");

        let parsed = parse_single_class("select-none").expect("Should parse");
        assert_eq!(parsed.property, "user-select");
        assert_eq!(parsed.value, "none");

        let parsed = parse_single_class("pointer-events-none").expect("Should parse");
        assert_eq!(parsed.property, "pointer-events");
        assert_eq!(parsed.value, "none");
    }

    #[test]
    fn test_parse_svg_utilities() {
        let parsed = parse_single_class("fill-red-500").expect("Should parse");
        assert_eq!(parsed.property, "fill");
        assert_eq!(parsed.value, "#ef4444");

        let parsed = parse_single_class("stroke-black").expect("Should parse");
        assert_eq!(parsed.property, "stroke");
        assert_eq!(parsed.value, "#000");

        let parsed = parse_single_class("stroke-2").expect("Should parse");
        assert_eq!(parsed.property, "stroke-width");
        assert_eq!(parsed.value, "2");
    }

    #[test]
    fn test_peer_variants() {
        let parsed = parse_single_class("peer-hover:bg-blue-500").expect("Should parse");
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::PeerHover);

        let selector = parsed.variants[0].to_selector();
        assert_eq!(selector.to_string(), ".peer:hover ~ &");
    }

    #[test]
    fn test_group_variants() {
        let parsed = parse_single_class("group-hover:bg-blue-500").expect("Should parse");
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::GroupHover);

        let selector = parsed.variants[0].to_selector();
        assert_eq!(selector.to_string(), "*[role=group]:hover &");
    }

    #[test]
    fn test_media_variants() {
        let parsed = parse_single_class("print:hidden").expect("Should parse");
        assert_eq!(parsed.variants.len(), 1);
        assert_eq!(parsed.variants[0], TailwindVariant::Print);

        let selector = parsed.variants[0].to_selector();
        if let StyleSelector::At { kind, query, .. } = selector {
            assert_eq!(kind, css::style_selector::AtRuleKind::Media);
            assert_eq!(query, "print");
        } else {
            panic!("Expected At selector");
        }
    }

    #[test]
    fn test_accessibility_utilities() {
        let parsed = parse_single_class("sr-only").expect("Should parse");
        assert_eq!(parsed.property, "position");
        assert_eq!(parsed.value, "absolute");
    }

    #[test]
    fn test_empty_string() {
        let styles = parse_tailwind_to_styles("", None);
        assert!(styles.is_empty());
    }

    #[test]
    fn test_unknown_class() {
        let result = parse_single_class("unknown-class");
        assert!(result.is_none());
    }

    #[test]
    fn test_is_likely_tailwind_class_exact_matches() {
        assert!(is_likely_tailwind_class("flex"));
        assert!(is_likely_tailwind_class("grid"));
        assert!(is_likely_tailwind_class("hidden"));
        assert!(is_likely_tailwind_class("absolute"));
        assert!(is_likely_tailwind_class("truncate"));
        assert!(is_likely_tailwind_class("sr-only"));
    }

    // ==================== WAVE 1: TailwindVariant Tests ====================

    // Wave 1.1: Pseudo-class variant selectors (lines 104-131)
    #[rstest]
    #[case(TailwindVariant::FocusVisible, "&:focus-visible")]
    #[case(TailwindVariant::FocusWithin, "&:focus-within")]
    #[case(TailwindVariant::Visited, "&:visited")]
    #[case(TailwindVariant::Enabled, "&:enabled")]
    #[case(TailwindVariant::Checked, "&:checked")]
    #[case(TailwindVariant::Indeterminate, "&:indeterminate")]
    #[case(TailwindVariant::Default, "&:default")]
    #[case(TailwindVariant::Required, "&:required")]
    #[case(TailwindVariant::Valid, "&:valid")]
    #[case(TailwindVariant::Invalid, "&:invalid")]
    #[case(TailwindVariant::InRange, "&:in-range")]
    #[case(TailwindVariant::OutOfRange, "&:out-of-range")]
    #[case(TailwindVariant::PlaceholderShown, "&:placeholder-shown")]
    #[case(TailwindVariant::Autofill, "&:autofill")]
    #[case(TailwindVariant::ReadOnly, "&:read-only")]
    #[case(TailwindVariant::FirstChild, "&:first-child")]
    #[case(TailwindVariant::LastChild, "&:last-child")]
    #[case(TailwindVariant::OnlyChild, "&:only-child")]
    #[case(TailwindVariant::OddChild, "&:nth-child(odd)")]
    #[case(TailwindVariant::EvenChild, "&:nth-child(even)")]
    #[case(TailwindVariant::FirstOfType, "&:first-of-type")]
    #[case(TailwindVariant::LastOfType, "&:last-of-type")]
    #[case(TailwindVariant::OnlyOfType, "&:only-of-type")]
    #[case(TailwindVariant::Empty, "&:empty")]
    #[case(TailwindVariant::Target, "&:target")]
    #[case(TailwindVariant::Open, "&[open]")]
    fn test_variant_to_selector_pseudo_classes(
        #[case] variant: TailwindVariant,
        #[case] expected: &str,
    ) {
        assert_eq!(
            variant.to_selector(),
            StyleSelector::Selector(expected.to_string())
        );
    }

    // Wave 1.2: Pseudo-element variant selectors (lines 133-141)
    #[rstest]
    #[case(TailwindVariant::Placeholder, "&::placeholder")]
    #[case(TailwindVariant::Before, "&::before")]
    #[case(TailwindVariant::After, "&::after")]
    #[case(TailwindVariant::Selection, "&::selection")]
    #[case(TailwindVariant::Marker, "&::marker")]
    #[case(TailwindVariant::FirstLetter, "&::first-letter")]
    #[case(TailwindVariant::FirstLine, "&::first-line")]
    #[case(TailwindVariant::Backdrop, "&::backdrop")]
    #[case(TailwindVariant::File, "&::file-selector-button")]
    fn test_variant_to_selector_pseudo_elements(
        #[case] variant: TailwindVariant,
        #[case] expected: &str,
    ) {
        assert_eq!(
            variant.to_selector(),
            StyleSelector::Selector(expected.to_string())
        );
    }

    // Wave 1.3: Group/Peer variant selectors (lines 143-151)
    #[rstest]
    #[case(TailwindVariant::GroupFocus, "*[role=group]:focus &")]
    #[case(TailwindVariant::GroupActive, "*[role=group]:active &")]
    #[case(TailwindVariant::GroupDisabled, "*[role=group]:disabled &")]
    #[case(TailwindVariant::PeerFocus, ".peer:focus ~ &")]
    #[case(TailwindVariant::PeerActive, ".peer:active ~ &")]
    #[case(TailwindVariant::PeerDisabled, ".peer:disabled ~ &")]
    #[case(TailwindVariant::PeerChecked, ".peer:checked ~ &")]
    #[case(TailwindVariant::PeerInvalid, ".peer:invalid ~ &")]
    fn test_variant_to_selector_group_peer(
        #[case] variant: TailwindVariant,
        #[case] expected: &str,
    ) {
        assert_eq!(
            variant.to_selector(),
            StyleSelector::Selector(expected.to_string())
        );
    }

    // Wave 1.4: Media variant selectors (lines 153-162)
    #[rstest]
    #[case(TailwindVariant::Screen, "screen")]
    #[case(TailwindVariant::Portrait, "(orientation: portrait)")]
    #[case(TailwindVariant::Landscape, "(orientation: landscape)")]
    #[case(TailwindVariant::MotionReduce, "(prefers-reduced-motion: reduce)")]
    #[case(TailwindVariant::MotionSafe, "(prefers-reduced-motion: no-preference)")]
    #[case(TailwindVariant::ContrastMore, "(prefers-contrast: more)")]
    #[case(TailwindVariant::ContrastLess, "(prefers-contrast: less)")]
    #[case(TailwindVariant::ForcedColors, "(forced-colors: active)")]
    fn test_variant_to_selector_media_queries(
        #[case] variant: TailwindVariant,
        #[case] expected_query: &str,
    ) {
        if let StyleSelector::At { kind, query, .. } = variant.to_selector() {
            assert_eq!(kind, css::style_selector::AtRuleKind::Media);
            assert_eq!(query, expected_query);
        } else {
            panic!("Expected At selector for {:?}", variant);
        }
    }

    // Wave 1.4 continued: Direction variants (lines 161-162)
    #[rstest]
    #[case(TailwindVariant::Rtl, "[dir=rtl] &")]
    #[case(TailwindVariant::Ltr, "[dir=ltr] &")]
    fn test_variant_to_selector_direction(
        #[case] variant: TailwindVariant,
        #[case] expected: &str,
    ) {
        assert_eq!(
            variant.to_selector(),
            StyleSelector::Selector(expected.to_string())
        );
    }

    // Wave 1.4: from_prefix tests for untested variants (lines 220-230)
    #[rstest]
    #[case("screen", TailwindVariant::Screen)]
    #[case("portrait", TailwindVariant::Portrait)]
    #[case("landscape", TailwindVariant::Landscape)]
    #[case("motion-reduce", TailwindVariant::MotionReduce)]
    #[case("motion-safe", TailwindVariant::MotionSafe)]
    #[case("contrast-more", TailwindVariant::ContrastMore)]
    #[case("contrast-less", TailwindVariant::ContrastLess)]
    #[case("forced-colors", TailwindVariant::ForcedColors)]
    #[case("rtl", TailwindVariant::Rtl)]
    #[case("ltr", TailwindVariant::Ltr)]
    fn test_from_prefix_media_variants(#[case] prefix: &str, #[case] expected: TailwindVariant) {
        assert_eq!(TailwindVariant::from_prefix(prefix), Some(expected));
    }

    // ==================== WAVE 2: Edge Cases & Arbitrary Values ====================

    // Wave 2.1: combine_selectors with At-rule (lines 292-295)
    #[test]
    fn test_combine_selectors_at_rule_with_hover() {
        let parsed = parse_single_class("print:hover:bg-blue-500").expect("Should parse");
        assert_eq!(parsed.variants.len(), 2);
        assert_eq!(parsed.variants[0], TailwindVariant::Print);
        assert_eq!(parsed.variants[1], TailwindVariant::Hover);

        let static_style = parsed.to_static_style();
        let selector = static_style.selector().expect("Should have selector");
        // Should combine into At rule with nested hover selector
        if let StyleSelector::At {
            kind,
            query,
            selector: nested,
        } = selector
        {
            assert_eq!(*kind, css::style_selector::AtRuleKind::Media);
            assert_eq!(query, "print");
            assert!(nested.is_some());
        } else {
            panic!("Expected At selector");
        }
    }

    // Wave 2.2: is_valid_tailwind_value edge cases (lines 816, 825, 859, 864-866)
    #[test]
    fn test_has_tailwind_classes_arbitrary_syntax() {
        // Line 816: arbitrary value syntax detection
        assert!(has_tailwind_classes("w-[100px]"));
        assert!(has_tailwind_classes("bg-[#ff0000]"));
        assert!(has_tailwind_classes("p-[calc(100%-20px)]"));
    }

    #[test]
    fn test_is_valid_tailwind_value_empty() {
        // Line 825: empty value should return false
        assert!(!is_valid_tailwind_value(""));
    }

    #[test]
    fn test_is_valid_tailwind_value_size_keywords() {
        // Line 859: size keywords
        assert!(is_valid_tailwind_value("xs"));
        assert!(is_valid_tailwind_value("sm"));
        assert!(is_valid_tailwind_value("md"));
        assert!(is_valid_tailwind_value("lg"));
        assert!(is_valid_tailwind_value("xl"));
        assert!(is_valid_tailwind_value("2xl"));
        assert!(is_valid_tailwind_value("3xl"));
    }

    #[test]
    fn test_is_valid_tailwind_value_fractions() {
        // Lines 864-866: fraction values
        assert!(is_valid_tailwind_value("1/2"));
        assert!(is_valid_tailwind_value("1/3"));
        assert!(is_valid_tailwind_value("2/3"));
        assert!(is_valid_tailwind_value("1/4"));
        assert!(is_valid_tailwind_value("3/4"));
    }

    // Wave 2.3: parse_arbitrary_value extended tests (lines 1057-1084)
    #[rstest]
    #[case("border-[#ff0000]", "border-color", "#ff0000")]
    #[case("opacity-[0.5]", "opacity", "0.5")]
    #[case("z-[999]", "z-index", "999")]
    #[case("font-[Arial]", "font-family", "Arial")]
    #[case("tracking-[0.2em]", "letter-spacing", "0.2em")]
    #[case("leading-[2]", "line-height", "2")]
    #[case("duration-[500ms]", "transition-duration", "500ms")]
    #[case("delay-[200ms]", "transition-delay", "200ms")]
    #[case("aspect-[16/9]", "aspect-ratio", "16/9")]
    #[case("columns-[3]", "columns", "3")]
    #[case("basis-[200px]", "flex-basis", "200px")]
    fn test_parse_arbitrary_values_extended(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("scale-[1.5]", "transform", "scale(1.5)")]
    #[case("rotate-[30deg]", "transform", "rotate(30deg)")]
    #[case("translate-x-[50px]", "transform", "translateX(50px)")]
    #[case("translate-y-[50px]", "transform", "translateY(50px)")]
    #[case("skew-x-[10deg]", "transform", "skewX(10deg)")]
    #[case("skew-y-[10deg]", "transform", "skewY(10deg)")]
    fn test_parse_arbitrary_transform_values(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case(
        "grid-cols-[200px_1fr]",
        "grid-template-columns",
        "repeat(200px 1fr, minmax(0, 1fr))"
    )]
    #[case(
        "grid-rows-[auto_1fr]",
        "grid-template-rows",
        "repeat(auto 1fr, minmax(0, 1fr))"
    )]
    #[case("col-span-[2]", "grid-column", "span 2 / span 2")]
    #[case("row-span-[3]", "grid-row", "span 3 / span 3")]
    fn test_parse_arbitrary_grid_values(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("blur-[10px]", "filter", "blur(10px)")]
    #[case("brightness-[1.2]", "filter", "brightness(1.2)")]
    #[case("contrast-[1.5]", "filter", "contrast(1.5)")]
    #[case("saturate-[2]", "filter", "saturate(2)")]
    #[case("backdrop-blur-[5px]", "backdrop-filter", "blur(5px)")]
    fn test_parse_arbitrary_filter_values(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ==================== WAVE 3: Layout & Flex/Grid Utilities ====================

    // Wave 3.1: aspect-ratio utilities (lines 1198-1204)
    #[rstest]
    #[case("aspect-auto", "aspect-ratio", "auto")]
    #[case("aspect-square", "aspect-ratio", "1 / 1")]
    #[case("aspect-video", "aspect-ratio", "16 / 9")]
    fn test_parse_aspect_ratio_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.2: columns utilities (lines 1209-1226)
    #[rstest]
    #[case("columns-auto", "columns", "auto")]
    #[case("columns-3xs", "columns", "16rem")]
    #[case("columns-2xs", "columns", "18rem")]
    #[case("columns-xs", "columns", "20rem")]
    #[case("columns-sm", "columns", "24rem")]
    #[case("columns-md", "columns", "28rem")]
    #[case("columns-lg", "columns", "32rem")]
    #[case("columns-xl", "columns", "36rem")]
    #[case("columns-2xl", "columns", "42rem")]
    #[case("columns-3xl", "columns", "48rem")]
    #[case("columns-4xl", "columns", "56rem")]
    #[case("columns-5xl", "columns", "64rem")]
    #[case("columns-6xl", "columns", "72rem")]
    #[case("columns-7xl", "columns", "80rem")]
    fn test_parse_columns_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.3: break utilities (lines 1231-1252)
    #[rstest]
    #[case("break-after-auto", "break-after", "auto")]
    #[case("break-after-avoid", "break-after", "avoid")]
    #[case("break-after-all", "break-after", "all")]
    #[case("break-after-avoid-page", "break-after", "avoid-page")]
    #[case("break-after-page", "break-after", "page")]
    #[case("break-after-left", "break-after", "left")]
    #[case("break-after-right", "break-after", "right")]
    #[case("break-after-column", "break-after", "column")]
    #[case("break-before-auto", "break-before", "auto")]
    #[case("break-before-avoid", "break-before", "avoid")]
    #[case("break-before-all", "break-before", "all")]
    #[case("break-before-avoid-page", "break-before", "avoid-page")]
    #[case("break-before-page", "break-before", "page")]
    #[case("break-before-left", "break-before", "left")]
    #[case("break-before-right", "break-before", "right")]
    #[case("break-before-column", "break-before", "column")]
    #[case("break-inside-auto", "break-inside", "auto")]
    #[case("break-inside-avoid", "break-inside", "avoid")]
    #[case("break-inside-avoid-page", "break-inside", "avoid-page")]
    #[case("break-inside-avoid-column", "break-inside", "avoid-column")]
    fn test_parse_break_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.4: Position utilities (lines 1273-1304)
    #[rstest]
    #[case("top-0", "top", "0px")]
    #[case("top-4", "top", "1rem")]
    #[case("right-0", "right", "0px")]
    #[case("right-4", "right", "1rem")]
    #[case("bottom-0", "bottom", "0px")]
    #[case("bottom-4", "bottom", "1rem")]
    #[case("left-0", "left", "0px")]
    #[case("left-4", "left", "1rem")]
    #[case("inset-0", "inset", "0px")]
    #[case("inset-4", "inset", "1rem")]
    #[case("inset-x-0", "inset-inline", "0px")]
    #[case("inset-x-4", "inset-inline", "1rem")]
    #[case("inset-y-0", "inset-block", "0px")]
    #[case("inset-y-4", "inset-block", "1rem")]
    fn test_parse_position_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.5: Flex/Grid extended utilities (lines 1463-1529)
    #[rstest]
    #[case("basis-4", "flex-basis", "1rem")]
    #[case("basis-8", "flex-basis", "2rem")]
    #[case("order-1", "order", "1")]
    #[case("order-12", "order", "12")]
    #[case("grid-cols-1", "grid-template-columns", "repeat(1, minmax(0, 1fr))")]
    #[case("grid-cols-12", "grid-template-columns", "repeat(12, minmax(0, 1fr))")]
    #[case("grid-rows-1", "grid-template-rows", "repeat(1, minmax(0, 1fr))")]
    #[case("grid-rows-6", "grid-template-rows", "repeat(6, minmax(0, 1fr))")]
    #[case("col-start-1", "grid-column-start", "1")]
    #[case("col-start-auto", "grid-column-start", "auto")]
    #[case("col-end-1", "grid-column-end", "1")]
    #[case("col-end-auto", "grid-column-end", "auto")]
    #[case("row-start-1", "grid-row-start", "1")]
    #[case("row-start-auto", "grid-row-start", "auto")]
    #[case("row-end-1", "grid-row-end", "1")]
    #[case("row-end-auto", "grid-row-end", "auto")]
    #[case("gap-x-4", "column-gap", "1rem")]
    #[case("gap-y-4", "row-gap", "1rem")]
    fn test_parse_flex_grid_extended_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ==================== WAVE 4: Spacing, Sizing & Typography ====================

    // Wave 4.1: Logical spacing utilities (lines 1580-1656)
    #[rstest]
    #[case("ps-4", "padding-inline-start", "1rem")]
    #[case("pe-4", "padding-inline-end", "1rem")]
    #[case("ms-4", "margin-inline-start", "1rem")]
    #[case("me-4", "margin-inline-end", "1rem")]
    #[case("mr-4", "margin-right", "1rem")]
    #[case("mb-4", "margin-bottom", "1rem")]
    #[case("ml-4", "margin-left", "1rem")]
    #[case("space-x-4", "column-gap", "1rem")]
    #[case("space-y-4", "row-gap", "1rem")]
    #[case("space-x-reverse", "--tw-space-x-reverse", "1")]
    #[case("space-y-reverse", "--tw-space-y-reverse", "1")]
    fn test_parse_logical_spacing_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 4.2: Sizing variants (lines 1677-1789)
    #[rstest]
    #[case("min-w-min", "min-width", "min-content")]
    #[case("min-w-max", "min-width", "max-content")]
    #[case("min-w-fit", "min-width", "fit-content")]
    #[case("max-w-2xl", "max-width", "42rem")]
    #[case("max-w-3xl", "max-width", "48rem")]
    #[case("max-w-4xl", "max-width", "56rem")]
    #[case("max-w-5xl", "max-width", "64rem")]
    #[case("max-w-6xl", "max-width", "72rem")]
    #[case("max-w-7xl", "max-width", "80rem")]
    #[case("max-w-min", "max-width", "min-content")]
    #[case("max-w-max", "max-width", "max-content")]
    #[case("max-w-fit", "max-width", "fit-content")]
    #[case("max-w-prose", "max-width", "65ch")]
    #[case("max-w-screen-sm", "max-width", "640px")]
    #[case("max-w-screen-md", "max-width", "768px")]
    #[case("max-w-screen-lg", "max-width", "1024px")]
    #[case("max-w-screen-xl", "max-width", "1280px")]
    #[case("max-w-screen-2xl", "max-width", "1536px")]
    fn test_parse_width_utilities_extended(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("h-svh", "height", "100svh")]
    #[case("h-lvh", "height", "100lvh")]
    #[case("h-dvh", "height", "100dvh")]
    #[case("min-h-svh", "min-height", "100svh")]
    #[case("min-h-lvh", "min-height", "100lvh")]
    #[case("min-h-dvh", "min-height", "100dvh")]
    #[case("min-h-min", "min-height", "min-content")]
    #[case("min-h-max", "min-height", "max-content")]
    #[case("min-h-fit", "min-height", "fit-content")]
    #[case("max-h-svh", "max-height", "100svh")]
    #[case("max-h-lvh", "max-height", "100lvh")]
    #[case("max-h-dvh", "max-height", "100dvh")]
    #[case("max-h-min", "max-height", "min-content")]
    #[case("max-h-max", "max-height", "max-content")]
    #[case("max-h-fit", "max-height", "fit-content")]
    fn test_parse_height_utilities_extended(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 4.3: Typography extended (lines 1829-1940)
    #[rstest]
    #[case("text-start", "text-align", "start")]
    #[case("text-end", "text-align", "end")]
    #[case("hyphens-none", "hyphens", "none")]
    #[case("hyphens-manual", "hyphens", "manual")]
    #[case("hyphens-auto", "hyphens", "auto")]
    #[case("tracking-tighter", "letter-spacing", "-0.05em")]
    #[case("tracking-tight", "letter-spacing", "-0.025em")]
    #[case("tracking-normal", "letter-spacing", "0em")]
    #[case("tracking-wide", "letter-spacing", "0.025em")]
    #[case("tracking-wider", "letter-spacing", "0.05em")]
    #[case("tracking-widest", "letter-spacing", "0.1em")]
    fn test_parse_typography_extended_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("leading-none", "line-height", "1")]
    #[case("leading-tight", "line-height", "1.25")]
    #[case("leading-snug", "line-height", "1.375")]
    #[case("leading-normal", "line-height", "1.5")]
    #[case("leading-relaxed", "line-height", "1.625")]
    #[case("leading-loose", "line-height", "2")]
    #[case("leading-3", "line-height", ".75rem")]
    #[case("leading-4", "line-height", "1rem")]
    #[case("leading-5", "line-height", "1.25rem")]
    #[case("leading-6", "line-height", "1.5rem")]
    #[case("leading-7", "line-height", "1.75rem")]
    #[case("leading-8", "line-height", "2rem")]
    #[case("leading-9", "line-height", "2.25rem")]
    #[case("leading-10", "line-height", "2.5rem")]
    fn test_parse_leading_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 4.4: List styles & alignment (lines 1947-1965)
    #[rstest]
    #[case("list-inside", "list-style-position", "inside")]
    #[case("list-outside", "list-style-position", "outside")]
    #[case("list-none", "list-style-type", "none")]
    #[case("list-disc", "list-style-type", "disc")]
    #[case("list-decimal", "list-style-type", "decimal")]
    #[case("align-baseline", "vertical-align", "baseline")]
    #[case("align-top", "vertical-align", "top")]
    #[case("align-middle", "vertical-align", "middle")]
    #[case("align-bottom", "vertical-align", "bottom")]
    #[case("content-none", "content", "none")]
    fn test_parse_list_align_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ==================== WAVE 5: Backgrounds, Borders, Effects, etc. ====================

    // Wave 5.1: Background utilities (lines 1981-2051)
    #[rstest]
    #[case("bg-fixed", "background-attachment", "fixed")]
    #[case("bg-local", "background-attachment", "local")]
    #[case("bg-scroll", "background-attachment", "scroll")]
    #[case("bg-clip-border", "background-clip", "border-box")]
    #[case("bg-clip-padding", "background-clip", "padding-box")]
    #[case("bg-clip-content", "background-clip", "content-box")]
    #[case("bg-clip-text", "background-clip", "text")]
    #[case("bg-origin-border", "background-origin", "border-box")]
    #[case("bg-origin-padding", "background-origin", "padding-box")]
    #[case("bg-origin-content", "background-origin", "content-box")]
    fn test_parse_background_attachment_clip_origin(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("bg-bottom", "background-position", "bottom")]
    #[case("bg-center", "background-position", "center")]
    #[case("bg-left", "background-position", "left")]
    #[case("bg-left-bottom", "background-position", "left bottom")]
    #[case("bg-left-top", "background-position", "left top")]
    #[case("bg-right", "background-position", "right")]
    #[case("bg-right-bottom", "background-position", "right bottom")]
    #[case("bg-right-top", "background-position", "right top")]
    #[case("bg-top", "background-position", "top")]
    fn test_parse_background_position(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("bg-repeat", "background-repeat", "repeat")]
    #[case("bg-no-repeat", "background-repeat", "no-repeat")]
    #[case("bg-repeat-x", "background-repeat", "repeat-x")]
    #[case("bg-repeat-y", "background-repeat", "repeat-y")]
    #[case("bg-repeat-round", "background-repeat", "round")]
    #[case("bg-repeat-space", "background-repeat", "space")]
    #[case("bg-auto", "background-size", "auto")]
    #[case("bg-cover", "background-size", "cover")]
    #[case("bg-contain", "background-size", "contain")]
    #[case("bg-none", "background-image", "none")]
    fn test_parse_background_repeat_size(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case(
        "bg-gradient-to-t",
        "background-image",
        "linear-gradient(to top, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-tr",
        "background-image",
        "linear-gradient(to top right, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-r",
        "background-image",
        "linear-gradient(to right, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-br",
        "background-image",
        "linear-gradient(to bottom right, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-b",
        "background-image",
        "linear-gradient(to bottom, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-bl",
        "background-image",
        "linear-gradient(to bottom left, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-l",
        "background-image",
        "linear-gradient(to left, var(--tw-gradient-stops))"
    )]
    #[case(
        "bg-gradient-to-tl",
        "background-image",
        "linear-gradient(to top left, var(--tw-gradient-stops))"
    )]
    fn test_parse_background_gradient(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.2: Gradient stops (lines 2057-2068)
    #[rstest]
    #[case("from-red-500", "--tw-gradient-from", "#ef4444")]
    #[case("from-blue-500", "--tw-gradient-from", "#3b82f6")]
    #[case("via-red-500", "--tw-gradient-via", "#ef4444")]
    #[case("via-blue-500", "--tw-gradient-via", "#3b82f6")]
    #[case("to-red-500", "--tw-gradient-to", "#ef4444")]
    #[case("to-blue-500", "--tw-gradient-to", "#3b82f6")]
    fn test_parse_gradient_stops(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.3: Border corners/sides (lines 2081-2189)
    #[rstest]
    #[case("rounded-t-lg", "border-top-left-radius", "0.5rem")]
    #[case("rounded-r-lg", "border-top-right-radius", "0.5rem")]
    #[case("rounded-b-lg", "border-bottom-right-radius", "0.5rem")]
    #[case("rounded-l-lg", "border-bottom-left-radius", "0.5rem")]
    #[case("rounded-tl-lg", "border-top-left-radius", "0.5rem")]
    #[case("rounded-tr-lg", "border-top-right-radius", "0.5rem")]
    #[case("rounded-br-lg", "border-bottom-right-radius", "0.5rem")]
    #[case("rounded-bl-lg", "border-bottom-left-radius", "0.5rem")]
    fn test_parse_border_radius_corners(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("border-t-2", "border-top-width", "2px")]
    #[case("border-r-2", "border-right-width", "2px")]
    #[case("border-b-2", "border-bottom-width", "2px")]
    #[case("border-l-2", "border-left-width", "2px")]
    #[case("border-x-2", "border-inline-width", "2px")]
    #[case("border-y-2", "border-block-width", "2px")]
    fn test_parse_border_width_sides(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.4: Border styles, outline, ring, divide (lines 2211-2313)
    #[rstest]
    #[case("border-solid", "border-style", "solid")]
    #[case("border-dashed", "border-style", "dashed")]
    #[case("border-dotted", "border-style", "dotted")]
    #[case("border-double", "border-style", "double")]
    #[case("border-hidden", "border-style", "hidden")]
    #[case("border-none", "border-style", "none")]
    fn test_parse_border_styles(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("outline-none", "outline", "2px solid transparent")]
    #[case("outline", "outline-style", "solid")]
    #[case("outline-dashed", "outline-style", "dashed")]
    #[case("outline-dotted", "outline-style", "dotted")]
    #[case("outline-double", "outline-style", "double")]
    #[case("outline-0", "outline-width", "0px")]
    #[case("outline-1", "outline-width", "1px")]
    #[case("outline-2", "outline-width", "2px")]
    #[case("outline-4", "outline-width", "4px")]
    #[case("outline-8", "outline-width", "8px")]
    fn test_parse_outline_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("ring", "box-shadow", "0 0 0 3px var(--tw-ring-color)")]
    #[case("ring-0", "--tw-ring-offset-shadow", "0 0 #0000")]
    #[case("ring-1", "box-shadow", "0 0 0 1px var(--tw-ring-color)")]
    #[case("ring-2", "box-shadow", "0 0 0 2px var(--tw-ring-color)")]
    #[case("ring-inset", "--tw-ring-inset", "inset")]
    fn test_parse_ring_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("divide-x", "--tw-divide-x-reverse", "0")]
    #[case("divide-y", "--tw-divide-y-reverse", "0")]
    #[case("divide-x-2", "border-inline-width", "2px")]
    #[case("divide-y-2", "border-block-width", "2px")]
    #[case("divide-x-reverse", "--tw-divide-x-reverse", "1")]
    #[case("divide-y-reverse", "--tw-divide-y-reverse", "1")]
    fn test_parse_divide_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.5: Effects (lines 2328-2350)
    #[rstest]
    #[case("shadow-red-500", "--tw-shadow-color", "#ef4444")]
    #[case("shadow-blue-500", "--tw-shadow-color", "#3b82f6")]
    fn test_parse_shadow_color_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("bg-blend-normal", "background-blend-mode", "normal")]
    #[case("bg-blend-multiply", "background-blend-mode", "multiply")]
    #[case("bg-blend-screen", "background-blend-mode", "screen")]
    #[case("bg-blend-overlay", "background-blend-mode", "overlay")]
    fn test_parse_blend_mode_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.6: Filters & backdrop filters (lines 2365-2555)
    #[rstest]
    #[case("blur-none", "filter", "blur(0)")]
    #[case("blur-sm", "filter", "blur(4px)")]
    #[case("blur-md", "filter", "blur(12px)")]
    #[case("blur-xl", "filter", "blur(24px)")]
    #[case("blur-2xl", "filter", "blur(40px)")]
    #[case("blur-3xl", "filter", "blur(64px)")]
    fn test_parse_blur_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("brightness-0", "filter", "brightness(0)")]
    #[case("brightness-50", "filter", "brightness(.5)")]
    #[case("brightness-100", "filter", "brightness(1)")]
    #[case("brightness-150", "filter", "brightness(1.5)")]
    #[case("brightness-200", "filter", "brightness(2)")]
    fn test_parse_brightness_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("contrast-0", "filter", "contrast(0)")]
    #[case("contrast-50", "filter", "contrast(.5)")]
    #[case("contrast-100", "filter", "contrast(1)")]
    #[case("contrast-150", "filter", "contrast(1.5)")]
    #[case("contrast-200", "filter", "contrast(2)")]
    fn test_parse_contrast_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case(
        "drop-shadow",
        "filter",
        "drop-shadow(0 1px 2px rgb(0 0 0 / 0.1)) drop-shadow(0 1px 1px rgb(0 0 0 / 0.06))"
    )]
    #[case("drop-shadow-sm", "filter", "drop-shadow(0 1px 1px rgb(0 0 0 / 0.05))")]
    #[case(
        "drop-shadow-md",
        "filter",
        "drop-shadow(0 4px 3px rgb(0 0 0 / 0.07)) drop-shadow(0 2px 2px rgb(0 0 0 / 0.06))"
    )]
    #[case(
        "drop-shadow-lg",
        "filter",
        "drop-shadow(0 10px 8px rgb(0 0 0 / 0.04)) drop-shadow(0 4px 3px rgb(0 0 0 / 0.1))"
    )]
    #[case(
        "drop-shadow-xl",
        "filter",
        "drop-shadow(0 20px 13px rgb(0 0 0 / 0.03)) drop-shadow(0 8px 5px rgb(0 0 0 / 0.08))"
    )]
    #[case(
        "drop-shadow-2xl",
        "filter",
        "drop-shadow(0 25px 25px rgb(0 0 0 / 0.15))"
    )]
    #[case("drop-shadow-none", "filter", "drop-shadow(0 0 #0000)")]
    fn test_parse_drop_shadow_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("hue-rotate-0", "filter", "hue-rotate(0deg)")]
    #[case("hue-rotate-15", "filter", "hue-rotate(15deg)")]
    #[case("hue-rotate-30", "filter", "hue-rotate(30deg)")]
    #[case("hue-rotate-60", "filter", "hue-rotate(60deg)")]
    #[case("hue-rotate-90", "filter", "hue-rotate(90deg)")]
    #[case("hue-rotate-180", "filter", "hue-rotate(180deg)")]
    fn test_parse_hue_rotate_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("invert-0", "filter", "invert(0)")]
    #[case("invert", "filter", "invert(100%)")]
    #[case("saturate-0", "filter", "saturate(0)")]
    #[case("saturate-50", "filter", "saturate(.5)")]
    #[case("saturate-100", "filter", "saturate(1)")]
    #[case("saturate-150", "filter", "saturate(1.5)")]
    #[case("saturate-200", "filter", "saturate(2)")]
    #[case("sepia-0", "filter", "sepia(0)")]
    #[case("sepia", "filter", "sepia(100%)")]
    fn test_parse_filter_effects_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("backdrop-blur", "backdrop-filter", "blur(8px)")]
    #[case("backdrop-blur-sm", "backdrop-filter", "blur(4px)")]
    #[case("backdrop-blur-md", "backdrop-filter", "blur(12px)")]
    #[case("backdrop-blur-lg", "backdrop-filter", "blur(16px)")]
    #[case("backdrop-blur-xl", "backdrop-filter", "blur(24px)")]
    #[case("backdrop-blur-2xl", "backdrop-filter", "blur(40px)")]
    #[case("backdrop-blur-3xl", "backdrop-filter", "blur(64px)")]
    #[case("backdrop-blur-none", "backdrop-filter", "blur(0)")]
    fn test_parse_backdrop_blur_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("backdrop-brightness-0", "backdrop-filter", "brightness(0)")]
    #[case("backdrop-brightness-100", "backdrop-filter", "brightness(1)")]
    #[case("backdrop-contrast-0", "backdrop-filter", "contrast(0)")]
    #[case("backdrop-contrast-100", "backdrop-filter", "contrast(1)")]
    #[case("backdrop-grayscale-0", "backdrop-filter", "grayscale(0)")]
    #[case("backdrop-grayscale", "backdrop-filter", "grayscale(100%)")]
    #[case("backdrop-invert-0", "backdrop-filter", "invert(0)")]
    #[case("backdrop-invert", "backdrop-filter", "invert(100%)")]
    #[case("backdrop-opacity-0", "backdrop-filter", "opacity(0)")]
    #[case("backdrop-opacity-100", "backdrop-filter", "opacity(1)")]
    #[case("backdrop-saturate-0", "backdrop-filter", "saturate(0)")]
    #[case("backdrop-saturate-100", "backdrop-filter", "saturate(1)")]
    #[case("backdrop-sepia-0", "backdrop-filter", "sepia(0)")]
    #[case("backdrop-sepia", "backdrop-filter", "sepia(100%)")]
    fn test_parse_backdrop_filter_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.7: Transitions & animations (lines 2600-2618)
    #[rstest]
    #[case("delay-0", "transition-delay", "0s")]
    #[case("delay-75", "transition-delay", "75ms")]
    #[case("delay-100", "transition-delay", "100ms")]
    #[case("delay-150", "transition-delay", "150ms")]
    #[case("delay-200", "transition-delay", "200ms")]
    #[case("delay-300", "transition-delay", "300ms")]
    #[case("delay-500", "transition-delay", "500ms")]
    #[case("delay-700", "transition-delay", "700ms")]
    #[case("delay-1000", "transition-delay", "1000ms")]
    fn test_parse_delay_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("animate-none", "animation", "none")]
    #[case("animate-spin", "animation", "spin 1s linear infinite")]
    #[case(
        "animate-ping",
        "animation",
        "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite"
    )]
    #[case(
        "animate-pulse",
        "animation",
        "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite"
    )]
    #[case("animate-bounce", "animation", "bounce 1s infinite")]
    fn test_parse_animation_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.8: Transforms (lines 2630-2716)
    #[rstest]
    #[case("scale-x-0", "transform", "scaleX(0)")]
    #[case("scale-x-50", "transform", "scaleX(0.5)")]
    #[case("scale-x-100", "transform", "scaleX(1)")]
    #[case("scale-x-150", "transform", "scaleX(1.5)")]
    #[case("scale-y-0", "transform", "scaleY(0)")]
    #[case("scale-y-50", "transform", "scaleY(0.5)")]
    #[case("scale-y-100", "transform", "scaleY(1)")]
    #[case("scale-y-150", "transform", "scaleY(1.5)")]
    fn test_parse_scale_axis_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("rotate-0", "transform", "rotate(0deg)")]
    #[case("rotate-1", "transform", "rotate(1deg)")]
    #[case("rotate-2", "transform", "rotate(2deg)")]
    #[case("rotate-3", "transform", "rotate(3deg)")]
    #[case("rotate-6", "transform", "rotate(6deg)")]
    #[case("rotate-12", "transform", "rotate(12deg)")]
    #[case("rotate-90", "transform", "rotate(90deg)")]
    #[case("rotate-180", "transform", "rotate(180deg)")]
    fn test_parse_rotate_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("translate-y-4", "transform", "translateY(1rem)")]
    #[case("translate-y-px", "transform", "translateY(1px)")]
    #[case("translate-y-full", "transform", "translateY(100%)")]
    #[case("translate-y-1/2", "transform", "translateY(50%)")]
    fn test_parse_translate_y_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("skew-x-0", "transform", "skewX(0deg)")]
    #[case("skew-x-1", "transform", "skewX(1deg)")]
    #[case("skew-x-2", "transform", "skewX(2deg)")]
    #[case("skew-x-3", "transform", "skewX(3deg)")]
    #[case("skew-x-6", "transform", "skewX(6deg)")]
    #[case("skew-x-12", "transform", "skewX(12deg)")]
    #[case("skew-y-0", "transform", "skewY(0deg)")]
    #[case("skew-y-1", "transform", "skewY(1deg)")]
    #[case("skew-y-2", "transform", "skewY(2deg)")]
    #[case("skew-y-3", "transform", "skewY(3deg)")]
    #[case("skew-y-6", "transform", "skewY(6deg)")]
    #[case("skew-y-12", "transform", "skewY(12deg)")]
    fn test_parse_skew_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("origin-center", "transform-origin", "center")]
    #[case("origin-top", "transform-origin", "top")]
    #[case("origin-top-right", "transform-origin", "top right")]
    #[case("origin-right", "transform-origin", "right")]
    #[case("origin-bottom-right", "transform-origin", "bottom right")]
    #[case("origin-bottom", "transform-origin", "bottom")]
    #[case("origin-bottom-left", "transform-origin", "bottom left")]
    #[case("origin-left", "transform-origin", "left")]
    #[case("origin-top-left", "transform-origin", "top left")]
    fn test_parse_transform_origin_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.9: Interactivity (lines 2732-2842)
    #[rstest]
    #[case("accent-auto", "accent-color", "auto")]
    #[case("accent-red-500", "accent-color", "#ef4444")]
    #[case("appearance-none", "appearance", "none")]
    #[case("appearance-auto", "appearance", "auto")]
    #[case("caret-red-500", "caret-color", "#ef4444")]
    fn test_parse_interactivity_accent_caret(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("scroll-auto", "scroll-behavior", "auto")]
    #[case("scroll-smooth", "scroll-behavior", "smooth")]
    #[case("scroll-m-4", "scroll-margin", "1rem")]
    #[case("scroll-p-4", "scroll-padding", "1rem")]
    fn test_parse_scroll_behavior_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("snap-start", "scroll-snap-align", "start")]
    #[case("snap-end", "scroll-snap-align", "end")]
    #[case("snap-center", "scroll-snap-align", "center")]
    #[case("snap-align-none", "scroll-snap-align", "none")]
    #[case("snap-none", "scroll-snap-type", "none")]
    #[case("snap-x", "scroll-snap-type", "x var(--tw-scroll-snap-strictness)")]
    #[case("snap-y", "scroll-snap-type", "y var(--tw-scroll-snap-strictness)")]
    #[case(
        "snap-both",
        "scroll-snap-type",
        "both var(--tw-scroll-snap-strictness)"
    )]
    #[case("snap-mandatory", "--tw-scroll-snap-strictness", "mandatory")]
    #[case("snap-proximity", "--tw-scroll-snap-strictness", "proximity")]
    #[case("snap-normal", "scroll-snap-stop", "normal")]
    #[case("snap-always", "scroll-snap-stop", "always")]
    fn test_parse_snap_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("touch-auto", "touch-action", "auto")]
    #[case("touch-none", "touch-action", "none")]
    #[case("touch-pan-x", "touch-action", "pan-x")]
    #[case("touch-pan-y", "touch-action", "pan-y")]
    #[case("touch-manipulation", "touch-action", "manipulation")]
    fn test_parse_touch_action_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("will-change-auto", "will-change", "auto")]
    #[case("will-change-scroll", "will-change", "scroll-position")]
    #[case("will-change-contents", "will-change", "contents")]
    #[case("will-change-transform", "will-change", "transform")]
    fn test_parse_will_change_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.10: SVG utilities (lines 2853-2863)
    #[rstest]
    #[case("fill-none", "fill", "none")]
    #[case("stroke-none", "stroke", "none")]
    #[case("stroke-0", "stroke-width", "0")]
    #[case("stroke-1", "stroke-width", "1")]
    fn test_parse_svg_extended_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }
}
