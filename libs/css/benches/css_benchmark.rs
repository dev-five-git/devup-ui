use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use css::class_map::reset_class_map;
use css::debug::set_debug;
use css::disassemble_property;
use css::file_map::reset_file_map;
use css::optimize_multi_css_value::optimize_multi_css_value;
use css::optimize_value::optimize_value;
use css::set_prefix;
use css::style_selector::{get_selector_order, global_selector_order};
use css::utils::{to_camel_case, to_kebab_case};
use css::{merge_selector, sheet_to_classname};

fn reset_state() {
    reset_class_map();
    reset_file_map();
    set_debug(false);
    set_prefix(None);
}

fn bench_sheet_to_classname(c: &mut Criterion) {
    let mut group = c.benchmark_group("sheet_to_classname");

    group.bench_function("simple", |b| {
        b.iter(|| {
            reset_state();
            sheet_to_classname(
                black_box("background"),
                black_box(0),
                black_box(Some("red")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.bench_function("with_selector", |b| {
        b.iter(|| {
            reset_state();
            sheet_to_classname(
                black_box("background"),
                black_box(0),
                black_box(Some("red")),
                black_box(Some("hover")),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.bench_function("with_filename", |b| {
        b.iter(|| {
            reset_state();
            sheet_to_classname(
                black_box("background"),
                black_box(0),
                black_box(Some("red")),
                black_box(None),
                black_box(None),
                black_box(Some("test.tsx")),
            )
        });
    });

    group.bench_function("all_params", |b| {
        b.iter(|| {
            reset_state();
            sheet_to_classname(
                black_box("background"),
                black_box(0),
                black_box(Some("red")),
                black_box(Some("hover")),
                black_box(Some(1)),
                black_box(Some("test.tsx")),
            )
        });
    });

    group.bench_function("multiple_sequential", |b| {
        b.iter(|| {
            reset_state();
            sheet_to_classname("background", 0, Some("red"), None, None, None);
            sheet_to_classname("color", 0, Some("white"), None, None, None);
            sheet_to_classname("padding", 0, Some("16px"), None, None, None);
            sheet_to_classname("margin", 0, Some("8px"), None, None, None);
            sheet_to_classname("display", 0, Some("flex"), None, None, None);
            sheet_to_classname("align-items", 0, Some("center"), None, None, None);
            sheet_to_classname("justify-content", 0, Some("center"), None, None, None);
            sheet_to_classname("width", 0, Some("100%"), None, None, None);
            sheet_to_classname("height", 0, Some("50vh"), None, None, None);
            sheet_to_classname("border-radius", 0, Some("8px"), None, None, None);
        });
    });

    group.finish();
}

fn bench_optimize_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimize_value");

    group.bench_function("simple_keyword", |b| {
        b.iter(|| optimize_value(black_box("red")));
    });

    group.bench_function("simple_px", |b| {
        b.iter(|| optimize_value(black_box("14px")));
    });

    group.bench_function("zero_unit", |b| b.iter(|| optimize_value(black_box("0px"))));

    group.bench_function("rgba_color", |b| {
        b.iter(|| optimize_value(black_box("rgba(255, 0, 0, 0.5)")));
    });

    group.bench_function("translate", |b| {
        b.iter(|| optimize_value(black_box("translate(10px, 0px)")));
    });

    group.bench_function("complex_multi", |b| {
        b.iter(|| optimize_value(black_box("0px 0px 10px rgba(0,0,0,0.1)")));
    });

    group.bench_function("theme_var", |b| {
        b.iter(|| optimize_value(black_box("$primary")));
    });

    // Semicolon-terminated values exercise the SEMI_SUFFIXES strip loop. The
    // quoted-single case matches the LAST probe entry, so it walks every probe
    // in the loop — the worst case the `break` short-circuits.
    group.bench_function("semi_quoted", |b| {
        b.iter(|| optimize_value(black_box("'red;'")));
    });

    // CSS custom-property name used as a value: hits the `starts_with("--")`
    // var()-wrap branch (the only path that folds the `has_custom_prop` scan).
    group.bench_function("custom_prop_name", |b| {
        b.iter(|| optimize_value(black_box("--var-0")));
    });

    group.finish();
}

fn bench_optimize_multi_css_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimize_multi_css_value");

    group.bench_function("bare_family", |b| {
        b.iter(|| optimize_multi_css_value(black_box("Roboto")));
    });

    group.bench_function("quoted_with_space", |b| {
        b.iter(|| optimize_multi_css_value(black_box("'Roboto Hello', sans-serif")));
    });

    group.bench_function("comma_list", |b| {
        b.iter(|| optimize_multi_css_value(black_box("'A', 'B', 'C'")));
    });

    group.bench_function("url_value", |b| {
        b.iter(|| optimize_multi_css_value(black_box("url('/f.ttf')")));
    });

    group.finish();
}

fn bench_to_kebab_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("to_kebab_case");

    group.bench_function("backgroundColor", |b| {
        b.iter(|| to_kebab_case(black_box("backgroundColor")));
    });

    group.bench_function("borderRadius", |b| {
        b.iter(|| to_kebab_case(black_box("borderRadius")));
    });

    group.bench_function("justifyContent", |b| {
        b.iter(|| to_kebab_case(black_box("justifyContent")));
    });

    group.bench_function("WebkitTransform", |b| {
        b.iter(|| to_kebab_case(black_box("WebkitTransform")));
    });

    group.bench_function("simple_color", |b| {
        b.iter(|| to_kebab_case(black_box("color")));
    });

    // Non-ASCII property name: exercises the `chars()` verbatim-copy branch
    // (utils.rs:11-20) that never hits the ASCII-uppercase fast path, locking the
    // ASCII-only uppercase gate against a future Unicode regression.
    group.bench_function("non_ascii", |b| {
        b.iter(|| to_kebab_case(black_box("marginТоп")));
    });

    group.finish();
}

fn bench_merge_selector(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_selector");

    group.bench_function("no_selector", |b| {
        b.iter(|| merge_selector(black_box("a"), black_box(None)));
    });

    group.bench_function("hover", |b| {
        b.iter(|| merge_selector(black_box("a"), black_box(Some(&"hover".into()))));
    });

    group.bench_function("theme_dark", |b| {
        b.iter(|| merge_selector(black_box("a"), black_box(Some(&"theme-dark".into()))));
    });

    // Multi-`&` selector: exercises the `extra_amps` capacity branch (lib.rs:135)
    // that the 0-/1-`&` cases above never touch, so a future capacity regression
    // on the rare chained-`&` shape stays attributable.
    group.bench_function("multi_amp", |b| {
        b.iter(|| {
            merge_selector(
                black_box("a"),
                black_box(Some(&":root[data-theme=dark]:hover & &".into())),
            )
        });
    });

    group.finish();
}

fn bench_global_selector_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("global_selector_order");

    // Isolates the single-pass `SELECTOR_ORDER` scan feeding the
    // `sheet_create_css_selectors` comparator. Covers an early-table hit, a
    // late-table hit, another late hit, and a no-match suffix (walks the whole
    // table with no assignment) so future `SELECTOR_ORDER` scan changes are
    // attributable.
    group.bench_function("hover", |b| {
        b.iter(|| global_selector_order(black_box(":hover")));
    });

    group.bench_function("focus_visible", |b| {
        b.iter(|| global_selector_order(black_box(":focus-visible")));
    });

    group.bench_function("disabled", |b| {
        b.iter(|| global_selector_order(black_box(":disabled")));
    });

    group.bench_function("no_match", |b| {
        b.iter(|| global_selector_order(black_box("::-webkit-scrollbar")));
    });

    group.finish();
}

fn bench_get_selector_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_selector_order");

    // Per-comparison key source for the plain-selector decorate-sort in
    // `create_style_with_layers`. Covers the single-`&` table-hit fast path, a
    // multi-`&` / prefixed selector (whole-string fallback), a `" &"` group
    // suffix (the strip-suffix pseudo scan), and a no-match tail.
    group.bench_function("amp_hover", |b| {
        b.iter(|| get_selector_order(black_box("&:hover")));
    });

    group.bench_function("theme_focus", |b| {
        b.iter(|| get_selector_order(black_box(":root[data-theme=dark] &:focus")));
    });

    group.bench_function("group_suffix", |b| {
        b.iter(|| get_selector_order(black_box(":is([role=group]):hover &")));
    });

    group.bench_function("no_match", |b| {
        b.iter(|| get_selector_order(black_box("&.custom")));
    });

    group.finish();
}

fn bench_disassemble_property(c: &mut Criterion) {
    let mut group = c.benchmark_group("disassemble_property");

    // Runs per extracted property. Cover a mapped hit (borrowed `&'static`
    // slice), a plain kebab fallback, and a vendor-prefix fallback (the
    // single-buffer `-<kebab>` build). Fully drain the returned iterator so the
    // whole disassembly is measured, not just its construction.
    group.bench_function("mapped", |b| {
        b.iter(|| disassemble_property(black_box("margin")).count());
    });

    group.bench_function("kebab_fallback", |b| {
        b.iter(|| disassemble_property(black_box("backgroundColor")).count());
    });

    group.bench_function("vendor_prefix", |b| {
        b.iter(|| disassemble_property(black_box("WebkitTransform")).count());
    });

    group.finish();
}

fn bench_to_camel_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("to_camel_case");

    // Sibling of the covered `to_kebab_case`; used on the group/theme selector
    // path. Locks the split-based rebuild against regression.
    group.bench_function("background-color", |b| {
        b.iter(|| to_camel_case(black_box("background-color")));
    });

    group.bench_function("min-width", |b| {
        b.iter(|| to_camel_case(black_box("min-width")));
    });

    group.bench_function("color", |b| {
        b.iter(|| to_camel_case(black_box("color")));
    });

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_sheet_to_classname(c);
    bench_optimize_value(c);
    bench_optimize_multi_css_value(c);
    bench_to_kebab_case(c);
    bench_merge_selector(c);
    bench_global_selector_order(c);
    bench_get_selector_order(c);
    bench_disassemble_property(c);
    bench_to_camel_case(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
