use boa_engine::{Context, Source};
use extractor::{
    ExtractOption, ModuleResolver, ResolvedModule, extract_with_modules, extract_without_source_map,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

const BASE: &str = "import {styled} from '@devup-ui/react';const trace=[];const selected=true;const Base=styled.article({...((trace.push('controller'),selected)?{color:(trace.push('leaf'),'red')}:{}),color:'green'});export {Base};";

fn executable(filename: &str, source: &str, resolver: Option<&ModuleResolver>) -> String {
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    let output = match resolver {
        Some(resolver) => extract_with_modules(filename, source, option, false, resolver),
        None => extract_without_source_map(filename, source, option),
    }
    .unwrap_or_else(|error| panic!("module extraction failed: {error}"));
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &output.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let mut program = parsed.program;
    let mut imports = String::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier {
                    let helper = match specifier.imported.name().as_str() {
                        "jsx" => Some("jsx"),
                        "forwardRef" => Some("forwardRef"),
                        _ => None,
                    };
                    if let Some(helper) = helper {
                        write!(imports, "const {}={helper};", specifier.local.name).unwrap_or_else(
                            |error| panic!("helper binding formatting failed: {error}"),
                        );
                    }
                }
            }
        }
    }
    program.body.retain(|statement| {
        !matches!(
            statement,
            Statement::ImportDeclaration(_) | Statement::ExportNamedDeclaration(_)
        )
    });
    format!("{imports}{}", Codegen::new().build(&program).code)
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn captures_keep_defining_scope_when_imported_fixed_styles_extend(#[case] readable: bool) {
    // Given: overwritten styles retain module-local effects, with conflicting importer names.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, importer: &str| {
        (specifier == "./base" && importer == "/src/app.ts").then(|| ResolvedModule {
            path: "/src/base.ts".to_string(),
            code: BASE.to_string(),
        })
    };
    let base = executable("/src/base.ts", BASE, None);
    let app = executable(
        "/src/app.ts",
        "import {styled} from '@devup-ui/react';import {Base} from './base';const trace=[];const selected=false;const Card=styled(Base)({padding:'2px'});export {Card};",
        if readable { Some(&resolver) } else { None },
    );
    let script = format!(
        "const forwardRef=render=>({{render}});const jsx=(target,props)=>({{target,props}});const base=(()=>{{{base}return {{Base,trace}};}})();const app=(({{Base}})=>{{{app}return {{Card,trace}};}})(base);function render(target,props){{const e=target.render(props,props.ref);return typeof e.target==='string'?e:render(e.target,e.props);}}render(app.Card,{{ref:7,children:'child'}});JSON.stringify([base.trace,app.trace]);"
    );

    // When: real generated callbacks render the extension across isolated module closures.
    let result = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("module execution failed: {error}\n{script}"));
    let observed = result
        .as_string()
        .unwrap_or_else(|| panic!("module observation must be JSON"))
        .to_std_string_escaped();

    // Then: only the defining module observes controller/leaf evaluation, regardless of readability.
    assert_eq!(
        observed, r#"[["controller","leaf"],[]]"#,
        "readable={readable}\n{script}"
    );
}
use std::fmt::Write;

fn observe(script: &str) -> String {
    Context::default()
        .eval(Source::from_bytes(script))
        .unwrap_or_else(|error| panic!("module execution failed: {error}\n{script}"))
        .as_string()
        .unwrap_or_else(|| panic!("module observation must be JSON"))
        .to_std_string_escaped()
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn outer_effects_precede_definer_effects_when_fixed_import_renders_repeatedly(
    #[case] readable: bool,
    #[values(false, true)] overlap: bool,
) {
    // Given: both modules retain effects behind overwritten fixed declarations.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = BASE.replace(
        "trace.push('controller')",
        "(trace.push('controller'),order.push('A'))",
    );
    let base = executable("/src/base.ts", &source, None);
    let resolver = move |specifier: &str, importer: &str| {
        (specifier == "./base" && importer == "/src/app.ts").then(|| ResolvedModule {
            path: "/src/base.ts".to_string(),
            code: source.clone(),
        })
    };
    let color = if overlap { ",color:'blue'" } else { "" };
    let app = executable(
        "/src/app.ts",
        &format!(
            "import {{styled}} from '@devup-ui/react';import {{Base}} from './base';const trace=[];const Card=styled(Base)({{...((trace.push('controller'),order.push('B'),true)?{{margin:(trace.push('leaf'),'1px')}}:{{}}),margin:'2px',padding:'2px'{color}}});export {{Card}};"
        ),
        if readable { Some(&resolver) } else { None },
    );
    let script = format!(
        "const order=[];const forwardRef=render=>({{render}});const jsx=(target,props)=>({{target,props}});const base=(()=>{{{base}return {{Base,trace}};}})();const app=(({{Base}})=>{{{app}return {{Card,trace}};}})(base);function render(target,props){{const e=target.render(props,props.ref);return typeof e.target==='string'?e:render(e.target,e.props);}}const before=[base.trace.slice(),app.trace.slice()];const first=render(app.Card,{{ref:7,children:'first'}});const second=render(app.Card,{{ref:8,children:'second'}});JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children]),base.trace,app.trace,order]);"
    );

    // When: the emitted callbacks execute in their separate real module closures.
    let observed = observe(&script);

    // Then: outer B precedes inner A on every render without moving either closure.
    assert_eq!(
        observed,
        serde_json::json!([
            [[], []],
            [["article", 7, "first"], ["article", 8, "second"]],
            ["controller", "leaf", "controller", "leaf"],
            ["controller", "leaf", "controller", "leaf"],
            ["B", "A", "B", "A"]
        ])
        .to_string(),
        "readable={readable}, overlap={overlap}\n{script}"
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn definer_captures_stay_local_when_native_attrs_preclude_a_fixed_slot(#[case] readable: bool) {
    // Given: literal native attrs exclude fixed_native, but overwritten captures still matter.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = BASE.replace("styled.article(", "styled.article.attrs({title:'base'})(");
    let base = executable("/src/base.ts", &source, None);
    let resolver = move |specifier: &str, importer: &str| {
        (specifier == "./base" && importer == "/src/app.ts").then(|| ResolvedModule {
            path: "/src/base.ts".to_string(),
            code: source.clone(),
        })
    };
    let app = executable(
        "/src/app.ts",
        "import {styled} from '@devup-ui/react';import {Base} from './base';const trace=[];const Card=styled(Base)({padding:'2px'});export {Card};",
        if readable { Some(&resolver) } else { None },
    );
    let script = format!(
        "const forwardRef=render=>({{render}});const jsx=(target,props)=>({{target,props}});const base=(()=>{{{base}return {{Base,trace}};}})();const app=(({{Base}})=>{{{app}return {{Card,trace}};}})(base);function render(target,props){{const e=target.render(props,props.ref);return typeof e.target==='string'?e:render(e.target,e.props);}}const before=[base.trace.slice(),app.trace.slice()];const final=render(app.Card,{{ref:7,children:'child'}});JSON.stringify([before,final.target,final.props.title,final.props.ref,final.props.children,base.trace,app.trace]);"
    );

    // When: a real imported extension renders a capture-bearing native attrs definition.
    let observed = observe(&script);

    // Then: native attrs and definer-only effects survive, even without fixed-slot capability.
    assert_eq!(
        observed,
        serde_json::json!([
            [[], []],
            "article",
            "base",
            7,
            "child",
            ["controller", "leaf"],
            []
        ])
        .to_string(),
        "readable={readable}\n{script}"
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn custom_retarget_keeps_definer_captures_when_target_reads_component_props(
    #[case] captured: bool,
) {
    // Given: a custom retarget needs component forwarding, not native theme omissions.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = if captured {
        BASE
    } else {
        "import {styled} from '@devup-ui/react';const trace=[];const Base=styled.article({color:'green'});export {Base};"
    };
    let resolver = move |specifier: &str, importer: &str| {
        (specifier == "./base" && importer == "/src/app.ts").then(|| ResolvedModule {
            path: "/src/base.ts".to_string(),
            code: source.to_string(),
        })
    };
    let base = executable("/src/base.ts", source, None);
    let app = executable(
        "/src/app.ts",
        "import {styled} from '@devup-ui/react';import {forwardRef as makeTarget} from 'react';import {jsx as createElement} from 'react/jsx-runtime';import {Base} from './base';const trace=[];const Target=makeTarget((p,ref)=>createElement('aside',{title:p.theme?.tone,'data-custom':p.customProp,ref,children:p.children,className:p.className,style:p.style}));const Card=Base.withComponent(Target);export {Card};",
        Some(&resolver),
    );
    let script = format!(
        "const forwardRef=render=>({{render}});const jsx=(target,props)=>({{target,props}});const base=(()=>{{{base}return {{Base,trace}};}})();const app=(({{Base}})=>{{{app}return {{Card,trace}};}})(base);function render(target,props){{const e=target.render(props,props.ref);return typeof e.target==='string'?e:render(e.target,e.props);}}const final=render(app.Card,{{ref:7,children:'child',theme:{{tone:'caller'}},customProp:'custom'}});JSON.stringify([final.target,final.props.title,final.props['data-custom'],final.props.ref,final.props.children,base.trace,app.trace]);"
    );

    // When: real compiled modules render the imported custom withComponent target.
    let observed = observe(&script);

    // Then: forwarding and lexical ownership both survive the custom target.
    let trace = if captured {
        vec!["controller", "leaf"]
    } else {
        vec![]
    };
    assert_eq!(
        observed,
        serde_json::json!(["aside", "caller", "custom", 7, "child", trace, []]).to_string(),
        "captured={captured}\n{script}"
    );
}
