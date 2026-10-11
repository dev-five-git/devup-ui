use rstest::rstest;

use super::super::demand::select_consumer;
use super::{Allocator, Parser, SemanticBuilder, SourceType, names};
use crate::module_loader::{Mapped, demand::Demand};
use crate::ordinary_ve::execution::SelectedModule;
use crate::vanilla_extract::Stylesheet;

#[rstest]
#[case(
    "declare const value: string;\nexport {value};",
    "/render.ts:1:15: required input is erased by TypeScript. Fix: provide a runtime value"
)]
#[case(
    "using value=acquire();export {value};",
    "/render.ts:1:1: required resource declaration cannot run exactly. Fix: use a plain initializer"
)]
#[case(
    "await using value=acquire();export {value};",
    "/render.ts:1:1: required resource declaration cannot run exactly. Fix: use a plain initializer"
)]
#[case(
    "declare function value(): string;export {value};",
    "/render.ts:1:1: required callable is erased by TypeScript. Fix: provide a runtime function"
)]
fn consumer_render_rejects_selected_erased_or_resource_inputs(
    #[case] source: &str,
    #[case] expected: &str,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let demand = Demand::prefixed("value", &Demand::whole());
    let view = select_consumer(
        (&parsed.program, &built.semantic),
        &demand,
        ("/render.ts", "@vanilla-extract/css", None),
    );
    assert_eq!(names(&view.selection.units), vec!["value"]);

    let result = view.render(
        Stylesheet {
            filename: "/render.ts",
            code: source,
            source,
            edits: &[],
        },
        &crate::ExtractOption::default(),
    );

    assert_eq!(result.err(), Some(expected.into()));
}

#[test]
fn consumer_render_preserves_var_initialization_kind() -> Result<(), Box<dyn std::error::Error>> {
    let source = "export var value='violet';";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let demand = Demand::prefixed("value", &Demand::whole());
    let view = select_consumer(
        (&parsed.program, &built.semantic),
        &demand,
        ("/render.ts", "@vanilla-extract/css", None),
    );

    let rendered = view.render(
        Stylesheet {
            filename: "/render.ts",
            code: source,
            source,
            edits: &[],
        },
        &crate::ExtractOption::default(),
    )?;

    assert_eq!(
        rendered.mapped.code,
        "var value='violet';\nexport {value as \"value\"};\n"
    );
    assert_eq!(
        rendered
            .bindings
            .iter()
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>(),
        vec!["value"]
    );
    assert_eq!(rendered.native.len(), 0);
    Ok(())
}

#[test]
fn copying_selected_property_unit_omits_host_only_sibling() {
    let source = "export const value={kept:'violet',discard:window.name};";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let demand = Demand::prefixed("value", &Demand::prefixed("kept", &Demand::whole()));
    let view = select_consumer(
        (&parsed.program, &built.semantic),
        &demand,
        ("/render.ts", "@vanilla-extract/css", None),
    );
    assert_eq!(names(&view.selection.units), vec!["value"]);
    let mut mapped = Mapped::default();

    view.copy_unit(
        SelectedModule {
            stylesheet: Stylesheet {
                filename: "/render.ts",
                code: source,
                source,
                edits: &[],
            },
            selection: &view.selection,
        },
        &view.selection.units[0],
        &mut mapped,
    );

    assert_eq!(mapped.code, "value={kept:'violet',}");
}

#[test]
fn consumer_function_does_not_select_inactive_nested_callback_dependencies()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "const browser=window.document;export function value(){const unused=()=>browser;return 'violet'}";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let demand = Demand::prefixed("value", &Demand::whole());

    let view = select_consumer(
        (&parsed.program, &built.semantic),
        &demand,
        ("/render.ts", "@vanilla-extract/css", None),
    );

    assert_eq!(names(&view.selection.units), vec!["value"]);
    let rendered = view.render(
        Stylesheet {
            filename: "/render.ts",
            code: source,
            source,
            edits: &[],
        },
        &crate::ExtractOption::default(),
    )?;
    assert_eq!(
        rendered.mapped.code,
        "function value(){const unused=()=>browser;return 'violet'};\nexport {value as \"value\"};\n"
    );
    Ok(())
}
