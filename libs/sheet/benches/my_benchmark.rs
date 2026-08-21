#![allow(clippy::unwrap_used)]

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use css::class_map::reset_class_map;
use css::file_map::reset_file_map;
use css::style_selector::StyleSelector;
use extractor::extract_style::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, extract};
use rustc_hash::FxHashSet;
use sheet::StyleSheet;
use sheet::theme::{ColorTheme, Theme, Typography};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Duration;

fn make_large_theme() -> Theme {
    let mut theme = Theme::default();
    let mut default_colors = ColorTheme::default();
    let mut dark_colors = ColorTheme::default();

    for idx in 0..80 {
        let name = format!("color.{idx}");
        default_colors.add_color(&name, &format!("#{idx:02x}{idx:02x}{idx:02x}"));
        dark_colors.add_color(
            &name,
            &format!("#{:02x}{:02x}{:02x}", 255 - idx, 255 - idx, 255 - idx),
        );
    }
    theme.add_color_theme("default", default_colors);
    theme.add_color_theme("dark", dark_colors);

    for idx in 0..80 {
        theme.add_length(
            "default",
            &format!("space{idx}"),
            vec![
                Some(format!("{}px", idx + 1)),
                Some(format!("{}px", idx + 2)),
                None,
                Some(format!("{}px", idx + 4)),
            ],
        );
        theme.add_shadow(
            "default",
            &format!("shadow{idx}"),
            vec![
                Some(format!("0 {}px {}px #0003", idx + 1, idx + 2)),
                None,
                Some(format!("0 {}px {}px #0004", idx + 2, idx + 4)),
            ],
        );
    }

    for idx in 0..40 {
        theme.add_typography(
            &format!("type{idx}"),
            vec![
                Some(Typography::new(
                    Some("Inter".to_string()),
                    Some(format!("{}px", 12 + idx)),
                    Some("600".to_string()),
                    Some("1.4".to_string()),
                    None,
                )),
                Some(Typography::new(
                    Some("Inter".to_string()),
                    Some(format!("{}px", 14 + idx)),
                    Some("700".to_string()),
                    Some("1.5".to_string()),
                    Some("0.01em".to_string()),
                )),
            ],
        );
    }

    theme
}

fn make_large_single_theme() -> Theme {
    // Single color variant only (`default`), so `to_css` takes the `single_theme` color
    // fast path (no `light-dark()` partner, no `default_optimized_colors` map).
    let mut theme = Theme::default();
    let mut default_colors = ColorTheme::default();
    for idx in 0..80 {
        let name = format!("color.{idx}");
        default_colors.add_color(&name, &format!("#{idx:02x}{idx:02x}{idx:02x}"));
    }
    theme.add_color_theme("default", default_colors);
    theme
}

fn make_large_sheet() -> StyleSheet {
    let mut sheet = StyleSheet::default();
    sheet.set_theme(make_large_theme());
    for idx in 0..300 {
        let class_name = format!("c{idx}");
        let property = if idx % 2 == 0 { "color" } else { "background" };
        let value = if idx % 3 == 0 { "$color.1" } else { "red" };
        sheet.add_property(&class_name, property, 0, value, None, None, Some("app.tsx"));
    }
    sheet
}

fn make_selector_sheet() -> StyleSheet {
    let mut sheet = StyleSheet::default();
    sheet.set_theme(make_large_theme());
    // Real selector variants so the `create_style` sort path exercises
    // `StyleSelector::Ord` / `get_selector_order` (pseudo, theme, group).
    let selectors = [
        StyleSelector::from("hover"),
        StyleSelector::from("focusVisible"),
        StyleSelector::from("active"),
        StyleSelector::from("theme-dark"),
        StyleSelector::from("group-hover"),
    ];
    for idx in 0..300 {
        let class_name = format!("s{idx}");
        let property = if idx % 2 == 0 { "color" } else { "background" };
        let value = if idx % 3 == 0 { "$color.1" } else { "red" };
        let selector = &selectors[idx % selectors.len()];
        let level = (idx % 4) as u8;
        sheet.add_property(
            &class_name,
            property,
            level,
            value,
            Some(selector),
            None,
            Some("app.tsx"),
        );
    }
    sheet
}

fn make_update_option() -> ExtractOption {
    ExtractOption {
        package: "@devup-ui/react".to_string(),
        css_dir: "@devup-ui/react".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::new(),
    }
}

/// Build a realistic `FxHashSet<ExtractStyleValue>` covering every `update_styles`
/// match arm. `ExtractDynamicStyle`/`ExtractKeyframes` live in `pub(super)` modules
/// so they can only be produced through the public `extract` entry point; the
/// remaining `Static` theme-token and `Typography` variants are added directly.
fn make_update_styles_set() -> FxHashSet<ExtractStyleValue> {
    // `extract` populates the global class/file maps; reset so this one-time
    // (untimed) build is deterministic and does not leak into the timed loop.
    reset_class_map();
    reset_file_map();
    let output = extract(
        "app.tsx",
        r"import {Box,Flex,keyframes} from '@devup-ui/react'
keyframes({from:{opacity:0,transform:'scale(0.5)'},to:{opacity:1,transform:'scale(1)'}});
const a = <Flex gap={2}>
  <Box bg={dynamicColor} color={themeColor} w={widthVar} h={heightVar} />
  <Box p={spacing} m={margin} _hover={{bg: hoverColor}} borderRadius={radius} />
</Flex>",
        make_update_option(),
    )
    .unwrap();
    let mut styles = output.styles;

    for idx in 0..40usize {
        let token = idx % 80;
        // Static theme-color tokens (default `CssVariable` resolution).
        styles.insert(ExtractStyleValue::Static(ExtractStaticStyle::new(
            if idx % 2 == 0 { "color" } else { "background" },
            &format!("$color.{token}"),
            (idx % 4) as u8,
            None,
        )));
        // `FirstValue` length token — resolves via `get_default_length_value`.
        styles.insert(ExtractStyleValue::Static(
            ExtractStaticStyle::new("width", &format!("$space{token}"), 0, None)
                .with_theme_token_resolution(ThemeTokenResolution::FirstValue),
        ));
        // `FirstValue` shadow token — resolves via `get_default_shadow_value`.
        styles.insert(ExtractStyleValue::Static(
            ExtractStaticStyle::new("box-shadow", &format!("$shadow{token}"), 0, None)
                .with_theme_token_resolution(ThemeTokenResolution::FirstValue),
        ));
        // Typography variant (no-op arm, still walked each iteration).
        styles.insert(ExtractStyleValue::Typography(format!("type{}", idx % 40)));
    }
    styles
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("theme_to_css_large", |b| {
        let theme = make_large_theme();
        b.iter(|| black_box(theme.to_css()));
    });

    c.bench_function("theme_to_css_single_theme", |b| {
        let theme = make_large_single_theme();
        b.iter(|| black_box(theme.to_css()));
    });

    c.bench_function("sheet_create_css_large", |b| {
        let sheet = make_large_sheet();
        b.iter(|| black_box(sheet.create_css(Some("app.tsx"), false)));
    });

    c.bench_function("sheet_create_css_selectors", |b| {
        let sheet = make_selector_sheet();
        b.iter(|| black_box(sheet.create_css(Some("app.tsx"), false)));
    });

    c.bench_function("create_interface", |b| {
        let mut sheet = StyleSheet::default();
        sheet.set_theme(make_large_theme());
        b.iter(|| {
            black_box(sheet.create_interface(
                black_box("@devup-ui/react"),
                black_box("DevupColorTheme"),
                black_box("DevupTypography"),
                black_box("DevupLength"),
                black_box("DevupShadows"),
                black_box("DevupTheme"),
            ))
        });
    });

    c.bench_function("sheet_add_property", |b| {
        b.iter_batched(
            || {
                let mut sheet = StyleSheet::default();
                sheet.set_theme(make_large_theme());
                sheet
            },
            |mut sheet| {
                for idx in 0..300 {
                    let class_name = format!("c{idx}");
                    let property = if idx % 2 == 0 { "color" } else { "background" };
                    let value = if idx % 3 == 0 { "$color.1" } else { "red" };
                    black_box(sheet.add_property(
                        black_box(&class_name),
                        black_box(property),
                        black_box(0),
                        black_box(value),
                        black_box(None),
                        black_box(None),
                        black_box(Some("app.tsx")),
                    ));
                }
            },
            BatchSize::SmallInput,
        );
    });

    // `update_styles` mutates the GLOBAL class/file naming maps, so a measured call
    // is only deterministic when it starts from an empty registry. Resetting INSIDE
    // the timed routine (not setup) makes every iteration an identical cold insert
    // regardless of its position in the criterion batch — resetting in setup instead
    // leaves only the first call per batch cold and the rest warm, and that cold/warm
    // mix shifts with batch size (unstable medians). The reset is two `HashMap::clear`
    // calls (~ns) next to a ~kElem insert, so it does not distort the signal.
    // `SmallInput` keeps the themed-sheet setup batched (not interleaved with each
    // timed call), matching the stable `sheet_add_property` structure; the longer
    // measurement time keeps the median resilient to transient host load.
    let mut group = c.benchmark_group("sheet_update_styles");
    group.measurement_time(Duration::from_secs(10));
    group.bench_function("single", |b| {
        let styles = make_update_styles_set();
        b.iter_batched(
            || {
                let mut sheet = StyleSheet::default();
                sheet.set_theme(make_large_theme());
                sheet
            },
            |mut sheet| {
                reset_class_map();
                reset_file_map();
                black_box(sheet.update_styles(black_box(&styles), black_box("app.tsx"), true));
            },
            BatchSize::SmallInput,
        );
    });
    group.bench_function("per_file", |b| {
        let styles = make_update_styles_set();
        b.iter_batched(
            || {
                let mut sheet = StyleSheet::default();
                sheet.set_theme(make_large_theme());
                sheet
            },
            |mut sheet| {
                reset_class_map();
                reset_file_map();
                black_box(sheet.update_styles(black_box(&styles), black_box("app.tsx"), false));
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
