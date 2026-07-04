use criterion::{Criterion, criterion_group, criterion_main};
use css::style_selector::StyleSelector;
use sheet::StyleSheet;
use sheet::theme::{ColorTheme, Theme, Typography};
use std::hint::black_box;

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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("theme_to_css_large", |b| {
        let theme = make_large_theme();
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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
