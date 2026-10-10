use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

use crate::ExtractOption;
use crate::module_loader::{ModuleLoader, Unit, module_script};

pub(super) fn validate(filename: &str, code: &str) -> Result<(), Box<dyn std::error::Error>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::from_path(filename)?).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{code}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{code}");
    Ok(())
}

pub(super) fn exported_identity_and_class(
    filename: &str,
    code: &str,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    validate(filename, code)?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::from_path(filename)?).parse();
    for statement in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = statement {
            assert_eq!(import.source.value.as_str(), "@devup-ui/react/devup-ui.css");
            assert_eq!(import.specifiers.iter().flatten().count(), 0);
        }
    }
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    let unit = Unit::generated(filename, code);
    let mut loader = ModuleLoader::new(None, &option);
    let entry = module_script(&unit, &mut loader, true)?;
    let exports = entry
        .exports
        .iter()
        .map(|(name, binding)| Ok(format!("{}:({binding})", serde_json::to_string(name)?)))
        .collect::<Result<Vec<_>, serde_json::Error>>()?
        .join(",");
    let body = loader.script(&entry).text;
    let script = format!(
        "(()=>{{{body}\nconst __wrapper_exports__={{{exports}}};return JSON.stringify([__wrapper_exports__.loop.self===__wrapper_exports__.loop,__wrapper_exports__.box]);}})()"
    );
    let value = Context::default()
        .eval(Source::from_bytes(script.as_bytes()))
        .map_err(|error| error.to_string())?;
    let serialized = value
        .as_string()
        .ok_or("expected serialized predicate and class")?
        .to_std_string_escaped();
    Ok(serde_json::from_str::<(bool, String)>(&serialized)?)
}
