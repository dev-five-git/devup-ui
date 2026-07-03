use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use css::class_map::reset_class_map;
use css::debug::set_debug;
use css::file_map::reset_file_map;
use css::optimize_multi_css_value::optimize_multi_css_value;
use css::optimize_value::optimize_value;
use css::set_prefix;
use css::utils::to_kebab_case;
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

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_sheet_to_classname(c);
    bench_optimize_value(c);
    bench_optimize_multi_css_value(c);
    bench_to_kebab_case(c);
    bench_merge_selector(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
