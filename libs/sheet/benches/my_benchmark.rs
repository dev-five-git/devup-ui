use criterion::{Criterion, criterion_group, criterion_main};
use regex::Regex;
use std::hint::black_box;
use std::sync::LazyLock;

static VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\w+").unwrap());

fn convert_theme_variable_value_a(value: &str) -> String {
    if value.contains("$") {
        VAR_RE
            .replace_all(value, |caps: &regex::Captures| {
                format!("var(--{})", &caps[0][1..])
            })
            .to_string()
    } else {
        value.to_string()
    }
}

fn convert_theme_variable_value_b(value: &str) -> String {
    VAR_RE
        .replace_all(value, |caps: &regex::Captures| {
            format!("var(--{})", &caps[0][1..])
        })
        .to_string()
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("convert_theme_variable_value_a", |b| {
        b.iter(|| {
            convert_theme_variable_value_a(black_box("$primary"));
            convert_theme_variable_value_a(black_box("red"));
            convert_theme_variable_value_a(black_box("solid 2px red"));
            convert_theme_variable_value_a(black_box("solid 2px $primary"));
        })
    });

    c.bench_function("convert_theme_variable_value_b", |b| {
        b.iter(|| {
            convert_theme_variable_value_b(black_box("$primary"));
            convert_theme_variable_value_b(black_box("red"));
            convert_theme_variable_value_b(black_box("solid 2px red"));
            convert_theme_variable_value_b(black_box("solid 2px $primary"));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
