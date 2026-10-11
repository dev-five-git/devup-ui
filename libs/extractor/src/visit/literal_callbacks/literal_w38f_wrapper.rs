use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};
use rstest::rstest;
use serial_test::serial;
use std::path::Path;

#[path = "literal_w38j_c1.rs"]
mod literal_w38j_c1;

#[path = "literal_w38k_baselines.rs"]
mod literal_w38k_baselines;
#[path = "literal_w38k_c1c_pending.rs"]
mod literal_w38k_c1c_pending;
#[path = "literal_w38k_completion.rs"]
mod literal_w38k_completion;
#[path = "literal_w38k_defaults.rs"]
mod literal_w38k_defaults;
#[path = "literal_w38k_destructured.rs"]
mod literal_w38k_destructured;
#[path = "literal_w38k_generic.rs"]
mod literal_w38k_generic;
#[path = "literal_w38k_mixin_current.rs"]
mod literal_w38k_mixin_current;
#[path = "literal_w38k_normalization.rs"]
mod literal_w38k_normalization;
#[path = "literal_w38k_scopes.rs"]
mod literal_w38k_scopes;
#[path = "literal_w38k_spread.rs"]
mod literal_w38k_spread;
#[path = "literal_w38k_suppliers.rs"]
mod literal_w38k_suppliers;
#[path = "literal_w38k_support.rs"]
mod literal_w38k_support;
#[path = "literal_w38k_terminal.rs"]
mod literal_w38k_terminal;
#[path = "literal_w38k_throws.rs"]
mod literal_w38k_throws;
#[path = "literal_w38k_validation.rs"]
mod literal_w38k_validation;

#[path = "literal_w38l_c1c_controls.rs"]
mod literal_w38l_c1c_controls;
#[path = "literal_w38l_c1c_default.rs"]
mod literal_w38l_c1c_default;
#[path = "literal_w38l_c1c_diagnostics.rs"]
mod literal_w38l_c1c_diagnostics;
#[path = "literal_w38l_c1c_runtime.rs"]
mod literal_w38l_c1c_runtime;
#[path = "literal_w38l_c1c_source.rs"]
mod literal_w38l_c1c_source;
#[path = "literal_w38l_c1c_supplier.rs"]
mod literal_w38l_c1c_supplier;

#[path = "literal_w38m_abrupt.rs"]
mod literal_w38m_abrupt;
#[path = "literal_w38m_effects.rs"]
mod literal_w38m_effects;
#[path = "literal_w38m_metadata.rs"]
mod literal_w38m_metadata;
#[path = "literal_w38m_precedence.rs"]
mod literal_w38m_precedence;
#[path = "literal_w38m_preservation.rs"]
mod literal_w38m_preservation;
#[path = "literal_w38m_rejections.rs"]
mod literal_w38m_rejections;
#[path = "literal_w38m_runtime.rs"]
mod literal_w38m_runtime;
#[path = "literal_w38m_source.rs"]
mod literal_w38m_source;
#[path = "literal_w38m_supplier.rs"]
mod literal_w38m_supplier;
#[path = "literal_w38m_support.rs"]
mod literal_w38m_support;

#[path = "literal_w38m_scalar_effects.rs"]
mod literal_w38m_scalar_effects;
#[path = "literal_w38m_scalar_observe.rs"]
mod literal_w38m_scalar_observe;
#[path = "literal_w38m_scalar_oracle.rs"]
mod literal_w38m_scalar_oracle;
#[path = "literal_w38m_scalar_precedence.rs"]
mod literal_w38m_scalar_precedence;
#[path = "literal_w38m_scalar_source.rs"]
mod literal_w38m_scalar_source;
#[path = "literal_w38m_scalar_throw.rs"]
mod literal_w38m_scalar_throw;

fn compile(source: &str) -> Result<crate::ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::debug::set_debug(true);
    let result = crate::extract("a.tsx", source, crate::ExtractOption::default())
        .map_err(|error| error.to_string());
    css::debug::set_debug(false);
    result
}

fn evaluate(compiled: &str) -> (Vec<String>, Vec<String>) {
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, compiled, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = SemanticBuilder::new().build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let mut options = TransformOptions::default();
    options.jsx.runtime = JsxRuntime::Classic;
    options.jsx.pragma = Some("h".to_string());
    let transformed = Transformer::new(&allocator, Path::new("a.tsx"), &options)
        .build_with_scoping(semantic.semantic.into_scoping(), &mut parsed.program);
    assert_eq!(transformed.diagnostics.len(), 0);
    let code = Codegen::new().build(&parsed.program).code;
    let script = code
        .lines()
        .filter(|line| !line.starts_with("import "))
        .collect::<Vec<_>>()
        .join("\n");
    let script = format!(
        "const trace=[];const h=(type,props)=>({{type,props}});const createElement=h;{script}\nJSON.stringify([[a.props.className,b.props.className],trace]);"
    );
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    let json = value
        .to_string(&mut context)
        .unwrap_or_else(|error| panic!("{error}: {script}"))
        .to_std_string_escaped();
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{error}: {json}"))
}

#[rstest]
#[case("p=>p.active?2:3", (2,3), vec!["built","get","get"])]
#[case("function(p){return p.active?4:5}", (4,5), vec!["built","get","get"])]
#[case("p=>{if(p.active){trace.push('early');return 6}trace.push('tail');return 7}", (6,7), vec!["built","get","early","get","tail"])]
#[serial]
fn order_wrapper_when_source_selects_a_return_preserves_getter_trace(
    #[case] callback: &str,
    #[case] orders: (u8, u8),
    #[case] trace: Vec<&str>,
) {
    // Given: a real literal callback with distinct return orders and authored getters.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=render=>render;const Card=styled.div`style-order:${{{callback}}};color:red`;trace.push('built');const a=Card({{get active(){{trace.push('get');return true}}}},null);const b=Card({{get active(){{trace.push('get');return false}}}},null);"
    );
    // When: public extraction and the actual generated component renders execute.
    let compiled = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, actual_trace) = evaluate(&compiled.code);
    // Then: only the selected return applies, with one getter read per render.
    assert_eq!(actual_trace, trace);
    assert!(
        classes[0].contains(&format!("color-0-red--{}", orders.0)),
        "{}",
        classes[0]
    );
    assert!(
        classes[1].contains(&format!("color-0-red--{}", orders.1)),
        "{}",
        classes[1]
    );
}

#[rstest]
#[case("p=>({color:'red'})")]
#[case("p=>({...p})")]
#[case("p=>'not-an-order'")]
#[serial]
fn order_wrapper_when_source_candidate_is_nonnumeric_retains_located_rejection(
    #[case] callback: &str,
) {
    // Given: objects, authored spreads and strings are values, not envelope properties.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';\nconst Card=styled.div`style-order:${{{callback}}};color:red`;"
    );
    // When: the real public metadata path parses the candidate.
    let actual = compile(&source)
        .err()
        .unwrap_or_else(|| panic!("invalid order compiled"));
    // Then: it remains a located metadata error, never a coerced numeric class.
    assert!(actual.starts_with("a.tsx:2:"), "{actual}");
    assert!(actual.contains("styleOrder"), "{actual}");
}
