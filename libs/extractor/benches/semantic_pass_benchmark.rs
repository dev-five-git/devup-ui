#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What the `SemanticBuilder` passes cost the extractor.
//!
//! A file that imports `css` or `keyframes` pays one scope-analysis pass before
//! extraction (to read the style results it binds); a file with an error pays a
//! second one to report reads of what the build compiled away. Each group below
//! pairs two inputs that differ only in that pass, and `semantic_floor` is the
//! pass on its own, so the difference between the pair can be read against it.
//! Run: `cargo bench -p extractor --bench semantic_pass_benchmark`.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::hint::black_box;

use css::class_map::reset_class_map;
use css::debug::set_debug;
use css::file_map::reset_file_map;
use css::set_prefix;
use extractor::{ExtractOption, extract};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

fn make_option() -> ExtractOption {
    ExtractOption {
        package: "@devup-ui/react".to_string(),
        css_dir: "@devup-ui/react".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::new(),
    }
}

fn reset_state() {
    reset_class_map();
    reset_file_map();
    set_debug(false);
    set_prefix(None);
}

const BODY: &str = r#"
export const a = <Flex gap={2} direction="column">
  <Box bg="red" p={4} m={2} _hover={{bg: "blue"}} borderRadius="8px" />
  <Box bg={["red", "blue", "green"]} p={[1,2,3]} />
  <Text color="white" fontSize="14px" fontWeight="bold" _focus={{color: "red"}} />
  <Box display="flex" alignItems="center" justifyContent="center" w="100%" h="50vh" />
  <Box border="solid 1px red" boxShadow="0 4px 6px rgba(0,0,0,0.1)" transition="all 0.3s" />
</Flex>"#;

/// The same file with and without the import that turns the pass on
fn without_pass() -> String {
    format!("import {{Box, Flex, Text}} from '@devup-ui/react'{BODY}")
}

fn with_pass() -> String {
    format!("import {{Box, Flex, Text, css}} from '@devup-ui/react'{BODY}")
}

/// A file that reads what the build compiled away, so the error pass runs, and
/// the file that does not read it
fn with_error_pass() -> String {
    format!(
        "import {{Box, Flex, Text, css}} from '@devup-ui/react'\nexport const runtime = css{BODY}"
    )
}

fn bench_extract(c: &mut Criterion, group: &str, name: &str, input: &str) {
    c.benchmark_group(group).bench_function(name, |b| {
        b.iter(|| {
            reset_state();
            let _ = extract(black_box("test.tsx"), black_box(input), make_option());
        });
    });
}

/// The floor: parsing alone, and parsing plus the scope analysis the passes run
fn bench_floor(c: &mut Criterion, input: &str) {
    let mut floor = c.benchmark_group("semantic_floor");
    floor.bench_function("parse", |b| {
        b.iter(|| {
            let allocator = Allocator::default();
            let parsed = Parser::new(&allocator, black_box(input), SourceType::tsx()).parse();
            black_box(parsed.program.body.len())
        });
    });
    floor.bench_function("parse_and_semantic", |b| {
        b.iter(|| {
            let allocator = Allocator::default();
            let parsed = Parser::new(&allocator, black_box(input), SourceType::tsx()).parse();
            let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
            black_box(semantic.scoping().symbols_len())
        });
    });
    floor.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    let (off, on, error) = (without_pass(), with_pass(), with_error_pass());

    bench_floor(c, &on);

    bench_extract(c, "style_values_pass", "off", &off);
    bench_extract(c, "style_values_pass", "on", &on);
    bench_extract(c, "report_compiled_reads_pass", "off", &on);
    bench_extract(c, "report_compiled_reads_pass", "on", &error);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
