use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use super::{SelectedModule, Stylesheet, imports};
use crate::module_loader::Mapped;

fn option() -> crate::ExtractOption {
    crate::ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from_iter([(
            "@vanilla-extract/css".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..crate::ExtractOption::default()
    }
}

#[rstest]
#[case("using")]
#[case("await using")]
fn native_resource_declaration_reports_its_original_initializer(#[case] kind: &str) {
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';\n{kind} box=style({{color:'red'}});"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let stylesheet = Stylesheet {
        filename: "/resource.ts",
        code: &code,
        source: &code,
        edits: &[],
    };

    let result = super::source::build(stylesheet, &selection, &option());

    let column = kind.len() + 2;
    assert_eq!(
        result.err(),
        Some(format!(
            "/resource.ts:2:{column}: resource declarations cannot be captured exactly. Fix: use a plain exact initializer"
        ))
    );
}

#[rstest]
#[case("tokens", "tokens")]
#[case("tokens.colors.primary", "tokens.colors.primary")]
fn type_only_dependency_reports_its_demanded_read(#[case] read: &str, #[case] input: &str) {
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';\nimport type {{tokens}} from './tokens';\nconst tokens={{colors:{{primary:'red'}}}};\nconst box=style({{color:{read}}});"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let module = SelectedModule {
        stylesheet: Stylesheet {
            filename: "/type.ts",
            code: &code,
            source: &code,
            edits: &[],
        },
        selection: &selection,
    };

    let result = imports::write(module, &option(), &mut Mapped::default());

    assert_eq!(
        result,
        Err(format!(
            "/type.ts:4:24: required native input `{input}` comes from a type-only import. Fix: import a runtime value"
        ))
    );
}

#[test]
fn type_only_dependency_added_by_export_demand_reports_its_import_specifier() {
    let code = "import type {tokens} from './tokens';\nexport {tokens as palette};";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let demand = crate::module_loader::demand::Demand::prefixed(
        "palette",
        &crate::module_loader::demand::Demand::whole(),
    );
    let view = crate::ordinary_ve::selection::demand::select_reads(
        (&parsed.program, &built.semantic),
        (&demand, &[]),
        ("/export.ts", "@vanilla-extract/css", None),
    );
    let stylesheet = Stylesheet {
        filename: "/export.ts",
        code,
        source: code,
        edits: &[],
    };

    let result = view.render(stylesheet, &option());

    assert_eq!(result.err(), Some(
        "/export.ts:1:14: required native input `tokens` comes from a type-only import. Fix: import a runtime value".into()
    ));
}

#[test]
fn native_import_reports_original_specifier_when_alias_is_disabled() {
    let code =
        "import {style as paint} from '@vanilla-extract/css';const box=paint({ color:'red' });";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let module = SelectedModule {
        stylesheet: Stylesheet {
            filename: "/alias.ts",
            code,
            source: code,
            edits: &[],
        },
        selection: &selection,
    };

    let result = imports::write(
        module,
        &crate::ExtractOption::default(),
        &mut Mapped::default(),
    );

    assert_eq!(
        result,
        Err(
            "/alias.ts:1:9: native slice execution requires the enabled vanilla-extract alias"
                .into()
        )
    );
}

#[test]
fn copied_native_slice_contains_a_located_throw_for_api_value_escape() {
    let code = "import {style} from '@vanilla-extract/css';\nconst box=style({ color:style });";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let module = SelectedModule {
        stylesheet: Stylesheet {
            filename: "/escape.ts",
            code,
            source: code,
            edits: &[],
        },
        selection: &selection,
    };
    let mut mapped = Mapped::default();

    imports::copy(module, selection.units[0].span, &mut mapped);

    assert_eq!(
        mapped.code,
        concat!(
            "box=__ve_native_read__((style), __ve_api_style__)({ color:(function(){throw ",
            "\"/escape.ts:2:25: a native styling API escapes its exact initialization slice. Fix: ",
            "call the API in an exact initializer instead of exporting or handing off the API\";})() })"
        )
    );
}
