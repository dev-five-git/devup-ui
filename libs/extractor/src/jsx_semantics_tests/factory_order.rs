use super::*;
use crate::extract_style::ExtractStyleProperty;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};

// Bounded adapter of whole::plain_script: same parser/transformer/codegen/run,
// selectable actual JSX lowering and aliases for the automatic-runtime imports.
fn factory_script(source: &str, runtime: &str) -> String {
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx()).parse();
    assert!(
        !parsed.fatal_error && parsed.diagnostics.is_empty(),
        "{source}"
    );
    let mut program = parsed.program;
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut options = TransformOptions::default();
    options.jsx.runtime = if runtime == "automatic" {
        JsxRuntime::Automatic
    } else {
        JsxRuntime::Classic
    };
    if runtime != "automatic" {
        options.jsx.pragma = Some(
            if runtime == "pragma" {
                "h"
            } else {
                "React.createElement"
            }
            .into(),
        );
    }
    let transformed = Transformer::new(&arena, std::path::Path::new("factory.tsx"), &options)
        .build_with_scoping(scoping, &mut program);
    assert!(
        transformed.diagnostics.is_empty(),
        "{source}: {:?}",
        transformed.diagnostics
    );
    let mut aliases = String::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value.as_str() == Some("react/jsx-runtime")
        {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier {
                    assert!(matches!(
                        named.imported.to_string().as_str(),
                        "jsx" | "jsxs"
                    ));
                    aliases.push_str("const ");
                    aliases.push_str(named.local.name.as_str());
                    aliases.push_str(" = automatic;\n");
                }
            }
        }
    }
    let generated = oxc_codegen::Codegen::new().build(&program).code;
    aliases
        + &generated
            .lines()
            .filter(|line| !line.starts_with("import "))
            .map(|line| line.strip_prefix("export ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n")
}

#[test]
#[serial_test::serial]
fn factory_lookup_precedes_owned_style_evaluation() {
    struct RestoreDebug(bool);
    impl Drop for RestoreDebug {
        fn drop(&mut self) {
            css::debug::set_debug(self.0);
        }
    }
    let _debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(false);
    let prefix = "import {css} from '@emotion/react';";
    let mutable = "const props={get color(){trace.push('value');active='NEW';return 'purple';}};function f(props){return <div css={{color:props.color}}/>;}const result=observe(()=>f(props));";
    let throwing = "factoryThrows=true;const props={get color(){trace.push('value');throw valueError;}};function f(props){return <div css={{color:props.color}}/>;}const result=observe(()=>f(props));";
    let tdz = "const props={get color(){trace.push('value');return 'purple';}};function f(props){return <div css={{color:props.color}}/>;}const result=observe(()=>f(props));const h=automatic;";
    let initializer = "const props={get color(){trace.push('value');active='NEW';return 'purple';}};function f(props){const inner={color:props.color};return <div css={inner}/>;}const result=observe(()=>f(props));";
    let shorthand = "Object.defineProperty(globalThis,'position',{configurable:true,get(){trace.push('position');return 3;}});const result=observe(()=> <Box mx={position}/>);";
    let template = "const c={toString(){trace.push('coerce');active='NEW';return 'purple';}};function f(c){return <div css={{color:`${c}`}}/>;}const result=observe(()=>f(c));";
    let ordered = "const props={get color(){trace.push('value');return 'purple';}};function before(){trace.push('before');return 'id';}function after(){trace.push('after');return 'title';}function f(props){return <div id={before()} css={{color:props.color}} title={after()}/>;}const result=observe(()=>f(props));";
    let cases = [
        (
            "mutable",
            "classic",
            mutable,
            "ok",
            vec!["factory", "value", "call:OLD"],
        ),
        (
            "throw-identity",
            "classic",
            throwing,
            "factory",
            vec!["factory"],
        ),
        ("pragma-tdz", "pragma", tdz, "reference", vec![]),
        (
            "initializer",
            "classic",
            initializer,
            "ok",
            vec!["value", "factory", "call:NEW"],
        ),
        (
            "duplicate-shorthand",
            "classic",
            shorthand,
            "ok",
            vec!["factory", "position", "call:OLD"],
        ),
        (
            "dynamic-template",
            "classic",
            template,
            "ok",
            vec!["factory", "coerce", "call:OLD"],
        ),
        (
            "ordered-refusal",
            "classic",
            ordered,
            "ok",
            vec!["factory", "before", "value", "after", "call:OLD"],
        ),
        (
            "mutable",
            "automatic",
            mutable,
            "ok",
            vec!["value", "call:AUTO"],
        ),
        (
            "throw-identity",
            "automatic",
            throwing,
            "value",
            vec!["value"],
        ),
        (
            "dynamic-template",
            "automatic",
            template,
            "ok",
            vec!["coerce", "call:AUTO"],
        ),
        (
            "ordered-refusal",
            "automatic",
            ordered,
            "ok",
            vec!["before", "value", "after", "call:AUTO"],
        ),
    ];
    let prelude = "const trace=[];const factoryError={},valueError={};let active='OLD',factoryThrows=false;const Box='div';function automatic(type,props){trace.push('call:AUTO');return {type,props};}const React={get createElement(){trace.push('factory');if(factoryThrows)throw factoryError;const selected=active;return (type,props)=>{trace.push('call:'+selected);return {type,props};};}};function observe(invoke){try{invoke();return 'ok';}catch(error){return error===factoryError?'factory':error===valueError?'value':error instanceof ReferenceError?'reference':'other';}}";
    for (id, runtime, body, outcome, trace) in cases {
        let source = if id == "duplicate-shorthand" {
            format!("import {{Box}} from '@devup-ui/react';{body}")
        } else {
            format!("{prefix}{body}")
        };
        let authored_script = factory_script(&source, runtime);
        let authored_text = run(&format!(
            "{prelude}{authored_script}\nreturn JSON.stringify({{outcome:result,trace}});"
        ));
        let authored: serde_json::Value = serde_json::from_str(&authored_text)
            .unwrap_or_else(|error| panic!("{error}: {authored_text}"));
        assert_eq!(
            authored,
            serde_json::json!({"outcome":outcome,"trace":trace}),
            "authored {id}/{runtime}"
        );
        for single_css in [false, true] {
            reset_class_map();
            reset_file_map();
            let options = ExtractOption {
                single_css,
                import_aliases: HashMap::from([(
                    "@emotion/react".into(),
                    ImportAlias::NamedToNamed,
                )]),
                ..ExtractOption::default()
            };
            let output = extract("factory.tsx", &source, options)
                .unwrap_or_else(|error| panic!("{id}/{runtime}/{single_css}: {error}"));
            let script = factory_script(&output.code, runtime);
            let current_text = run(&format!(
                "{prelude}{script}\nreturn JSON.stringify({{outcome:result,trace}});"
            ));
            let current: serde_json::Value = serde_json::from_str(&current_text)
                .unwrap_or_else(|error| panic!("{error}: {current_text}"));
            if id == "duplicate-shorthand" {
                assert_eq!(
                    current["trace"]
                        .as_array()
                        .map(|rows| rows.iter().filter(|v| *v == "position").count()),
                    Some(1)
                );
            }
            assert_eq!(
                current, authored,
                "{id}/{runtime}/{single_css}: {}",
                output.code
            );
        }
    }
}

struct RestoreDebug(bool);
impl Drop for RestoreDebug {
    fn drop(&mut self) {
        css::debug::set_debug(self.0);
    }
}

#[test]
#[serial_test::serial]
fn public_dynamic_identifiers_match_emitted_style_expressions() {
    use oxc_ast_visit::{Visit, walk};
    struct Slots(Vec<(String, String)>);
    impl<'a> Visit<'a> for Slots {
        fn visit_object_property(&mut self, property: &oxc_ast::ast::ObjectProperty<'a>) {
            if let Some(key) = property.key.static_name().map(|key| {
                key.into_cow_str()
                    .unwrap_or_else(|| panic!("non-UTF-8 static property key"))
            }) && key.starts_with("--")
            {
                self.0.push((
                    key.into_owned(),
                    crate::utils::readable_code(&property.value),
                ));
            }
            walk::walk_object_property(self, property);
        }
    }
    let _restore_debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(false);
    for (body, expected) in [
        ("color: props.color", "props.color"),
        ("padding: gap * 4", "gap * 4"),
        ("color: `${c}`", "c"),
    ] {
        let source = format!(
            "import {{css}} from '@emotion/react';function f(props,gap,c){{return <div css={{{{{body}}}}}/>;}}"
        );
        reset_class_map();
        reset_file_map();
        let output = extract(
            "a.tsx",
            &source,
            ExtractOption {
                import_aliases: HashMap::from([(
                    "@emotion/react".into(),
                    ImportAlias::NamedToNamed,
                )]),
                ..ExtractOption::default()
            },
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let arena = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&arena, &output.code, oxc_span::SourceType::tsx()).parse();
        assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
        let mut slots = Slots(Vec::new());
        slots.visit_program(&parsed.program);
        assert_eq!(slots.0.len(), 1, "{}", output.code);
        assert_eq!(
            output
                .styles
                .iter()
                .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
                .count(),
            1
        );
        for style in &output.styles {
            if let ExtractStyleValue::Dynamic(style) = style {
                let StyleProperty::Variable {
                    variable_name,
                    identifier,
                    ..
                } = style.extract(Some("a.tsx"))
                else {
                    panic!("dynamic style has no variable");
                };
                assert!(identifier.contains(expected), "{identifier}");
                let expression =
                    oxc_parser::Parser::new(&arena, &identifier, oxc_span::SourceType::tsx())
                        .parse_expression()
                        .unwrap_or_else(|error| panic!("{error:?}"));
                assert_eq!(
                    slots.0,
                    vec![(variable_name, crate::utils::readable_code(&expression))]
                );
            }
        }
    }
}

#[test]
#[serial_test::serial]
fn rewriting_one_owner_preserves_other_sites_and_capture_fallbacks() {
    use oxc_ast_visit::{Visit, walk};
    use oxc_span::ContentEq;
    struct Slots(Vec<(String, String)>);
    impl<'a> Visit<'a> for Slots {
        fn visit_object_property(&mut self, property: &oxc_ast::ast::ObjectProperty<'a>) {
            if let Some(key) = property.key.static_name().map(|key| {
                key.into_cow_str()
                    .unwrap_or_else(|| panic!("non-UTF-8 static property key"))
            }) && key.starts_with("--")
            {
                self.0.push((
                    key.into_owned(),
                    crate::utils::readable_code(&property.value),
                ));
            }
            walk::walk_object_property(self, property);
        }
    }
    let _restore_debug = RestoreDebug(css::debug::is_debug());
    css::debug::set_debug(false);
    for body in [
        "const a=<div css={{color:first.color}}/>;const b=<Widget css={{backgroundColor:second.color}}/>;",
        "const b=<Widget css={{backgroundColor:second.color}}/>;const a=<div css={{color:first.color}}/>;",
        "const a=<div css={{color:first.color}}/>;const b=<div css={{color:second.color}}/>;",
    ] {
        for single_css in [true, false] {
            let source = format!(
                "import {{css}} from '@emotion/react';function f(first,second){{{body}return [a,b];}}"
            );
            reset_class_map();
            reset_file_map();
            let output = extract(
                "a.tsx",
                &source,
                ExtractOption {
                    single_css,
                    import_aliases: HashMap::from([(
                        "@emotion/react".into(),
                        ImportAlias::NamedToNamed,
                    )]),
                    ..ExtractOption::default()
                },
            )
            .unwrap_or_else(|error| panic!("{error}"));
            let arena = oxc_allocator::Allocator::default();
            let parsed =
                oxc_parser::Parser::new(&arena, &output.code, oxc_span::SourceType::tsx()).parse();
            assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
            let mut slots = Slots(Vec::new());
            slots.visit_program(&parsed.program);
            assert_eq!(slots.0.len(), 2, "{}", output.code);
            let guarded_first_reads = slots
                .0
                .iter()
                .filter(|(_, source)| {
                    let expression =
                        oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx())
                            .parse_expression()
                            .unwrap_or_else(|error| panic!("{error:?}"));
                    let Expression::CallExpression(call) = expression.get_inner_expression() else {
                        return false;
                    };
                    let Expression::ArrowFunctionExpression(arrow) =
                        call.callee.get_inner_expression()
                    else {
                        return false;
                    };
                    let Some(identifier) = arrow
                        .params
                        .items
                        .first()
                        .and_then(|parameter| parameter.pattern.get_binding_identifier())
                    else {
                        return false;
                    };
                    let expected = crate::style_slot_test_support::numeric_with_capture(
                        "first.color",
                        identifier.name.as_str(),
                    );
                    let guard =
                        oxc_parser::Parser::new(&arena, &expected, oxc_span::SourceType::tsx())
                            .parse_expression()
                            .unwrap_or_else(|error| panic!("{error:?}"));
                    expression
                        .get_inner_expression()
                        .content_eq(guard.get_inner_expression())
                })
                .count();
            assert_eq!(guarded_first_reads, 1);
            assert!(output.code.contains("second.color"));
            let mut public = Vec::new();
            for style in &output.styles {
                if let ExtractStyleValue::Dynamic(style) = style {
                    let StyleProperty::Variable {
                        variable_name,
                        identifier,
                        ..
                    } = style.extract(Some("a.tsx"))
                    else {
                        panic!("dynamic style has no variable");
                    };
                    let expression =
                        oxc_parser::Parser::new(&arena, &identifier, oxc_span::SourceType::tsx())
                            .parse_expression()
                            .unwrap_or_else(|error| panic!("{error:?}"));
                    public.push((variable_name, crate::utils::readable_code(&expression)));
                }
            }
            public.sort();
            slots.0.sort();
            assert_eq!(public.len(), 2);
            assert_eq!(public, slots.0, "{single_css}: {}", output.code);
        }
    }
}

#[path = "factory_spread.rs"]
mod spread_css;
