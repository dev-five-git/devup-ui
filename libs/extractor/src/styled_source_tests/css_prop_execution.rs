use super::*;
use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};
use boa_engine::{Context, Source};
use oxc_allocator::TakeIn;
use oxc_codegen::Codegen;
use oxc_semantic::SemanticBuilder;
use oxc_transformer::{TransformOptions, Transformer};
use serde_json::Value;

#[path = "call_choice_execution.rs"]
mod call_choice_execution;

#[path = "call_mixin_source_execution.rs"]
mod call_mixin_source_execution;

#[path = "template_binding_execution.rs"]
mod template_binding_execution;

#[path = "direct_template_execution.rs"]
mod direct_template_execution;

#[path = "callback_discarded_execution.rs"]
mod callback_discarded_execution;

#[path = "d1_projection_execution.rs"]
mod d1_projection_execution;

#[path = "d1_boundary_execution.rs"]
mod d1_boundary_execution;

#[path = "layer_transport_execution.rs"]
mod layer_transport_execution;

#[path = "callback_provenance_execution.rs"]
mod callback_provenance_execution;

#[path = "value_transport_execution.rs"]
mod value_transport_execution;

#[path = "ordinary_property_execution.rs"]
mod ordinary_property_execution;

#[path = "binding_execution.rs"]
mod binding_execution;

#[path = "scoping_execution.rs"]
mod scoping_execution;

#[path = "namespace_css_slots.rs"]
mod namespace_css_slots;

#[path = "css_boundary_forwarding.rs"]
mod css_boundary_forwarding;

#[path = "css_controller_execution.rs"]
mod css_controller_execution;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn layered_authored_styles_when_opaque_metadata_interleaves_ignore_every_raw_style_key(
    #[case] after: bool,
    #[values("joined-layer.ts", "joined-layer.tsx")] filename: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: raw metadata is opaque as a whole. Naming any of its keys in an
    // object around it is a build error, so the running code gives all of them.
    let opaque = "...raw";
    let props = if after {
        format!("className:classes,...{{p:1}},p:2,{opaque}")
    } else {
        format!("{opaque},className:classes,...{{p:1}},p:2")
    };
    let element = if filename.ends_with('x') {
        let raw = format!("{{{opaque}}}");
        let authored = concat!("className={classes} ", "{...{p:1}} p={2}");
        if after {
            format!("<Box {authored} {raw}/>")
        } else {
            format!("<Box {raw} {authored}/>")
        }
    } else {
        format!("jsx(Box,{{{props}}})")
    };
    let source = format!(
        "import {{Box}} from '@devup-ui/react';const classes=css({{'@layer':{{first:{{color:'red'}},last:{{color:'green'}}}}}},{{'@layer':{{first:{{color:'blue'}}}}}});const App=raw=>{element};"
    );
    // When: actual native props and active layer-owned atoms are observed.
    let result = execute_in_mode(
        filename,
        &source,
        "const layer={evil:{color:'raw'}};const raw={styleOrder:'abc',typography:'missing',selectors:{'&:hover':{color:'raw'}},css:{color:'raw'},'@layer':layer,p:{toString(){throw new Error('coerced');}}};const e=App(raw);JSON.stringify([[atoms(e).map(r=>[r[0],r[1],r[4]]).sort(),e.props.p===raw.p,e.props.selectors===raw.selectors,e.props.css===raw.css,e.props['@layer']===layer,e.props.styleOrder,e.props.typography,slots(e)],layerOrders]);",
        single_css,
    );
    // Then: known D1 retains its winners and raw metadata neither registers a layer nor normalizes data.
    assert_eq!(
        result[0],
        serde_json::json!([
            [
                ["color", "blue", "first"],
                ["color", "green", "last"],
                ["padding", "8px", null]
            ],
            true,
            true,
            true,
            true,
            "abc",
            "missing",
            []
        ])
    );
    assert_eq!(
        result[1],
        serde_json::json!([[filename, ["first", "last"]]])
    );
}

#[test]
#[serial]
fn unreadable_fixture_f_g_h_when_executed_keep_raw_copy_and_retained_behavior() {
    // Given: the conflicted snapshot's source forms, without depending on its renderer bytes.
    let source = "import {Box} from '@devup-ui/react';const f=(rest,handler,k)=>jsx(Box,{...rest,...{onClick:handler,[k]:1}});const g=(theme,key)=>jsx(Box,{bg:theme.colors[key],color:{a:'red',...theme.more}['b']});const h=Base=>styled.div(Base);const Card=h('caller');";
    // When: raw collisions, dynamic getters and repeated retained renders execute through their real surface.
    let result = execute(
        "joined-unreadable.ts",
        source,
        "const trace=[];const raw={p:9,className:'raw',style:{opacity: 0.4}};const handler=()=>0;const a=f(raw,handler,'title');const b=g({get colors(){trace.push('colors');return {x:'blue'};},get more(){trace.push('more');return {b:'green'};}},'x');const first=render({target:Card,props:{ref:7,children:'first'}});const second=render({target:Card,props:{ref:8,children:'second'}});JSON.stringify([[a.props.p,a.props.title,a.props.onClick===handler,a.props.className,a.props.style.opacity,values(a,'padding')],[values(b,'background'),values(b,'color'),trace],[first,second].map(e=>[e.target,e.props.ref,e.props.children,(e.props.className||'').split(/\\s+/).includes('caller'),slots(e)])]);",
    );
    // Then: no padding is invented from f, g reads once, and h retains forwarding across renders.
    assert_eq!(
        result,
        serde_json::json!([
            [9, 1, true, "raw", 0.4, []],
            [["blue"], ["green"], ["colors", "more"]],
            [
                ["div", 7, "first", true, []],
                ["div", 8, "second", true, []]
            ]
        ])
    );
    // Given: the entire literal a-j snapshot input, including its original imports and JSX.
    let source = r"import { Box, css, styled, keyframes } from '@devup-ui/react';
export const a = (base) => css(base);
export const b = (on) => css(on && { color: 'blue' });
export const c = css('color: green; margin: 0');
export const d = (on, base) => css(on ? base : { color: 'red' });
export const e = css({ ...null, ...false, ...undefined, ...(void 0), p: 1 });
export const f = (rest, handler, k) => <Box {...rest} {...{ onClick: handler, [k]: 1 }} />;
export const g = (theme, key) => <Box bg={theme.colors[key]} color={{ a: 'red', ...theme.more }['b']} />;
export const h = (Base) => styled.div(Base);
export const i = keyframes({ from: { opacity: 0 } } as const);
export const j = css(...{ bg: 'red' });";
    let _state = crate::test_state::TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    let output = extract("joined-exact.tsx", source, ExtractOption::default())
        .unwrap_or_else(|error| panic!("fixture must compile: {error}"));
    let bucket = crate::css_bucket("joined-exact.tsx", &ExtractOption::default());
    // When: collisions use native object-spread expectations, before serialization loses identities.
    let result = execute_extracted(
        ("joined-exact.tsx", bucket.as_deref()),
        &output,
        r"
const trace=[];const raw={p:{raw:true},className:'raw',style:{opacity:0.4},payload:{original:true}};const handler=()=>0;
const collisions=['title','p','className','style'].map(k=>{const expected={...raw,...{onClick:handler,[k]:1}};const actual=f(raw,handler,k);const names=typeof actual.props.className==='string'?actual.props.className.split(/\s+/):[];return [Object.keys(expected).every(key=>actual.props[key]===expected[key]),actual.props.onClick===handler,actual.props.payload===raw.payload,names.flatMap(name=>rules[name]?[rules[name]]:[]).filter(rule=>rule[0]==='padding')];});
const dynamic=g({get colors(){trace.push('colors');return {x:'blue'};},get more(){trace.push('more');return {b:'green'};}},'x');const Card=h('caller');const rendered=[7,8].map(ref=>render({target:Card,props:{ref,children:ref===7?'first':'second'}}));
JSON.stringify([collisions,[values(dynamic,'background'),values(dynamic,'color'),trace],rendered.map(e=>[e.target,e.props.ref,e.props.children,(e.props.className||'').split(/\s+/).includes('caller'),slots(e)])]);
",
    );
    // Then: raw identities/collisions, once-only getters and repeated h forwarding match the companion.
    assert_eq!(
        result,
        serde_json::json!([
            [
                [true, true, true, []],
                [true, true, true, []],
                [true, true, true, []],
                [true, true, true, []]
            ],
            [["blue"], ["green"], ["colors", "more"]],
            [
                ["div", 7, "first", true, []],
                ["div", 8, "second", true, []]
            ]
        ])
    );
}

pub(super) const CAPTURED: &str = "const trace=[];let selected=true;const Card=styled.article({...((trace.push('controller'),selected)?{color:(trace.push('leaf'),'red')}:{}),color:'green'});";

pub(super) fn element(filename: &str, props: &str) -> String {
    if filename.ends_with('x') {
        format!("<Card {props} />")
    } else {
        format!("jsx(Card,{{{props}}})")
    }
}

pub(super) fn css_props(filename: &str, css: &str, extra: &str) -> String {
    if filename.ends_with('x') {
        format!("css={{{css}}} {extra}")
    } else {
        format!("css:{css},{extra}")
    }
}

pub(super) fn execute(filename: &str, source: &str, observation: &str) -> Value {
    execute_in_mode(filename, source, observation, true)
}

fn execute_in_mode(filename: &str, source: &str, observation: &str, single_css: bool) -> Value {
    let _state = crate::test_state::TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    let option = ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    let bucket = crate::css_bucket(filename, &option);
    let output = extract(
        filename,
        &format!("import {{styled,css}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';{source}"),
        option,
    )
    .unwrap_or_else(|error| panic!("{filename}\n{source}\n{error}"));
    execute_extracted((filename, bucket.as_deref()), &output, observation)
}

pub(super) fn execute_extracted(
    (filename, bucket): (&str, Option<&str>),
    output: &crate::ExtractOutput,
    observation: &str,
) -> Value {
    let mut rules = std::collections::BTreeMap::new();
    let mut orders = Vec::new();
    for style in &output.styles {
        let rule = match style {
            crate::ExtractStyleValue::Static(style) => {
                let StyleProperty::ClassName(name) = style.extract(bucket) else {
                    panic!("static atom must have a class")
                };
                Some((
                    name,
                    serde_json::json!([
                        style.property(),
                        style.value(),
                        style.level(),
                        style
                            .selector()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                        style.layer()
                    ]),
                ))
            }
            crate::ExtractStyleValue::Dynamic(style) => {
                let StyleProperty::Variable {
                    class_name,
                    variable_name,
                    ..
                } = style.extract(bucket)
                else {
                    panic!("dynamic atom must have a variable")
                };
                Some((
                    class_name,
                    serde_json::json!([
                        style.property(),
                        format!("var({variable_name})"),
                        style.level(),
                        style
                            .selector()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                        style.layer
                    ]),
                ))
            }
            crate::ExtractStyleValue::LayerOrder(order) => {
                orders.push((&order.file, &order.layers));
                None
            }
            _ => None,
        };
        if let Some((name, rule)) = rule {
            rules.insert(name, rule);
        }
    }
    let arena = Allocator::default();
    let mut program = parse(&arena, filename, &output.code);
    if !filename.ends_with('x') {
        renders(&program);
    }
    let scoping = SemanticBuilder::new()
        .with_enum_eval(true)
        .build(&program)
        .semantic
        .into_scoping();
    let options = TransformOptions::default();
    let transformed = Transformer::new(&arena, std::path::Path::new(filename), &options)
        .build_with_scoping(scoping, &mut program);
    assert_eq!(transformed.diagnostics.len(), 0, "{}", output.code);
    let mut aliases = String::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier {
                    let imported = named.imported.to_string();
                    let local = named.local.name.as_str();
                    let value = match imported.as_str() {
                        "jsx" | "jsxs" | "jsxDEV" => "jsx",
                        "forwardRef" => "forwardRef",
                        _ => panic!("unexpected runtime import: {imported}"),
                    };
                    if local != value {
                        aliases.push_str("const ");
                        aliases.push_str(local);
                        aliases.push('=');
                        aliases.push_str(value);
                        aliases.push(';');
                    }
                }
            }
        }
    }
    for statement in &mut program.body {
        if let Statement::ExportDeclaration(export) = statement {
            *statement = Statement::from(export.declaration.take_in(&&arena));
        }
    }
    program.body.retain(|statement| {
        !matches!(statement, Statement::ImportDeclaration(_))
            && !matches!(statement, Statement::ExportNamedDeclaration(export)
                if export.specifiers.is_empty())
    });
    let code = Codegen::new().build(&program).code;
    let rules = serde_json::to_string(&rules)
        .unwrap_or_else(|error| panic!("atom map must serialize: {error}"));
    let orders = serde_json::to_string(&orders)
        .unwrap_or_else(|error| panic!("orders must serialize: {error}"));
    let script = format!(
        r"function forwardRef(render){{return {{render}};}}
function jsx(target,props){{return {{target,props}};}}
function require(name){{if(name==='react')return {{forwardRef}};if(name==='react/jsx-runtime')return {{jsx}};throw new Error(name);}}
function render(element){{return typeof element.target==='string'?element:render(element.target.render(element.props,element.props.ref));}}
const rules={rules};
const layerOrders={orders};
function atoms(element){{return (element.props.className||'').split(/\s+/).flatMap(name=>rules[name]?[rules[name]]:[]);}}
function values(element,property,level=0,selector=''){{return atoms(element).filter(rule=>rule[0]===property&&rule[2]===level&&rule[3]===selector).map(rule=>{{const value=rule[1];return value.startsWith('var(')?element.props.style[value.slice(4,-1)]:value;}}).sort();}}
function slots(element){{return Object.keys(element.props).filter(key=>key.startsWith('__devupCss'));}}
{aliases}{code}
{observation}"
    );
    let result = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("native CSS-prop execution: {error}\n{code}"));
    let json = result
        .as_string()
        .unwrap_or_else(|| panic!("observation must be JSON"))
        .to_std_string_escaped();
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{error}\n{json}\n{code}"))
}

#[rstest]
#[case::emotion("emotionCss", false, "1px")]
#[case::native("css", false, "4px")]
#[case::styled("styled.div", true, "4px")]
#[serial]
fn conditional_rule_arrays_keep_one_getter_read_and_consistent_branches(
    #[case] factory: &str,
    #[case] styled: bool,
    #[case] margin: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{css as emotionCss}} from '@emotion/react';const trace=[];const state={{get on(){{trace.push('read');return trace.length===1;}}}};const f=()=>{factory}(state.on?[{{color:'red'}},{{margin:1}}]:[{{color:'blue'}}]);"
    );
    let call = if styled {
        "render({target:f(),props:{}})"
    } else {
        "({target:'div',props:{className:f()}})"
    };
    let observation = format!(
        "const first={call};const firstTrace=trace.slice();const second={call};JSON.stringify([firstTrace,trace,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);"
    );
    let observed = execute_in_mode("test.ts", &source, &observation, single_css);
    assert_eq!(
        observed,
        serde_json::json!([["read"], ["read", "read"], ["red"], [margin], ["blue"], []]),
    );
}

#[path = "controller_evaluation.rs"]
mod controller_evaluation;

#[path = "props_function_evaluation.rs"]
mod props_function_evaluation;

#[path = "props_function_bindings.rs"]
mod props_function_bindings;

#[path = "props_function_order.rs"]
mod props_function_order;

#[path = "props_function_theme.rs"]
mod props_function_theme;

#[path = "implicit_theme_scope.rs"]
mod implicit_theme_scope;

#[path = "local_member_guard.rs"]
mod local_member_guard;

#[path = "callback_choice_execution.rs"]
mod callback_choice_execution;

#[path = "integration_semantics.rs"]
mod integration_semantics;

#[path = "integration_execution.rs"]
mod integration_execution;
