use super::*;
use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use rstest::rstest;

fn parse<'a>(ast: &AstBuilder<'a>, code: &str) -> Expression<'a> {
    let result = oxc_parser::Parser::new(
        ast.allocator(),
        ast.allocator().alloc_str(code),
        SourceType::tsx(),
    )
    .parse_expression();
    result.unwrap_or_else(|errors| panic!("{code}: {errors:?}"))
}

fn compiled<'a>(ast: &AstBuilder<'a>, component: Expression<'a>, input: Expression<'a>) -> String {
    let mut arrow = parse(ast, "()=>0");
    if let Expression::ArrowFunctionExpression(function) = &mut arrow {
        function.body = body(ast, component, input);
    }
    readable_code(&arrow)
}

fn execute(render: &str) -> String {
    let script = format!(
        "const trace=[];const Component=()=>null;const h=(type,props,...children)=>({{type:typeof type==='function'?type.name:type,props,children}});const x=17;const __devupRecord0=29;const o={{tag:'bound',method(){{return this===o?'bound':'unbound';}}}};let flag=true;function input(){{trace.push('input');return {{get a(){{trace.push('a');return 3;}},get b(){{trace.push('b');return 5;}},theme:7,keep:11,method:o.method}};}}const render={render};let value;try{{value=render();if(typeof value==='function')value=[value.name,value()];}}catch(error){{value=String(error);}}JSON.stringify([value,trace]);"
    );
    let script = crate::jsx_semantics_tests::whole::plain_script(&script);
    let value = Context::default().eval(Source::from_bytes(script.as_bytes()));
    let value = value.unwrap_or_else(|error| panic!("{error}: {script}"));
    let json = value.as_string();
    let json = json.unwrap_or_else(|| panic!("JSON observation"));
    json.to_std_string_escaped()
}

#[rstest]
#[case("__devupProps=>({a:__devupProps.a,b:__devupProps.b})")]
#[case("__devupProps=>((p)=>({a:p.a,b:p.b}))(__devupProps)")]
#[case("__devupProps=>((p)=>p.a)(__devupProps)")]
#[case("__devupProps=>((p)=>p)(__devupProps)")]
#[case("__devupProps=>[ ((p)=>({a:p.a,b:p.b}))(__devupProps), ((p)=>p.a)(__devupProps) ]")]
#[case("__devupProps=>(({theme:omitted,...dom})=>dom)(__devupProps)")]
#[case("__devupProps=>flag?(({a:omitted,...dom})=>dom)(__devupProps):null")]
#[case("__devupProps=>(({a:omitted,...dom})=>[dom,omitted])(__devupProps)")]
#[case("__devupProps=>(({a:omitted=4,...dom})=>dom)(__devupProps)")]
#[case("__devupProps=>(({[key()]:omitted,...dom})=>dom)(__devupProps)")]
#[case("__devupProps=>(({'a':omitted})=>omitted)(__devupProps)")]
#[case("__devupProps=>(([p])=>p)([__devupProps.a])")]
#[case("__devupProps=>((p=2)=>p)(undefined)")]
#[case("__devupProps=>((...p)=>p)(1)")]
#[case("__devupProps=>((p,q)=>[p,q])(__devupProps.a,__devupProps.b)")]
#[case("__devupProps=>((p)=>p)(...[__devupProps.a])")]
#[case("__devupProps=>((p)=>p)?.(__devupProps.a)")]
#[case("__devupProps=>((p)=>({a:p.a,b:p.b,method:p.method()}))(__devupProps)")]
#[case("__devupProps=>((p)=>p)(o.method)()")]
#[case("__devupProps=>delete ((p)=>p)(o.tag)")]
#[case("__devupProps=>typeof ((p)=>p)(missing)")]
#[case("__devupProps=>((p)=>p)(eval)('typeof __devupProps')")]
#[case("__devupProps=>({callback:((p)=>p)(()=>1)}).callback.name")]
#[case("__devupProps=>((p)=>p)(class { static { trace.push('class:'+this.name); } })")]
#[case("__devupProps=>((p)=>p)(function(){return 1;})")]
#[case("__devupProps=>((p)=>()=>p.a)(__devupProps)")]
#[case("__devupProps=>[1,2].map(value=>((p)=>()=>p)(value)).map(callback=>callback())")]
#[case("__devupProps=>[1,2].map(value=>((p)=>[p,p])(value))")]
#[case("__devupProps=>((p)=>p.a++ + p.a)(__devupProps)")]
#[case("__devupProps=>((p)=>({p}))(__devupProps.a)")]
#[case("__devupProps=>((p)=>({p,other:p}))(__devupProps.a)")]
#[case("__devupProps=>((p)=><p a={p.a}></p>)(Component)")]
#[case("__devupProps=>((Tag)=><Tag a={Tag.a}></Tag>)(Component)")]
#[case("__devupProps=>((p)=><p.Inner />)({'Inner':Component})")]
#[case("__devupProps=>((p)=>function(){return 1;})(__devupProps.a)")]
#[case("__devupProps=>((p)=>class { static value=p.a; })(__devupProps)")]
#[case("__devupProps=>{const a=__devupProps.a;return ((p)=>[p,p])(a);}")]
#[case(
    "__devupProps=>((__devupContext)=>((__devupContext)=>[__devupContext.a,__devupContext.b])(__devupContext))(__devupProps)"
)]
#[case(
    "__devupProps=>{const value=__devupProps.a;return ((__devupContext)=>{const second=__devupContext.b;return [value,second];})(__devupProps);}"
)]
#[case("({a=x})=>{const x=2;return a;}")]
#[case("__devupProps=>(({missing=x})=>{const x=2;return missing;})({})")]
#[case("__devupProps=>{var x=2;return x;}")]
#[case("__devupProps=>{'use strict';return __devupProps.a;}")]
#[case("__devupProps=>{const value=__devupProps.a;return;}")]
#[case("__devupProps=>{trace.push('body');return __devupProps.a;}")]
#[case("__devupProps=>{if(flag)return 2;return 3;}")]
#[case("__devupProps=>eval('typeof __devupRecord1')")]
#[case("__devupProps=>((p)=>eval('p.a'))(__devupProps)")]
#[case("__devupProps=>__devupRecord0+__devupProps.a")]
#[case("__devupProps=>(()=>{const v=__devupProps.a;return v;})()")]
#[case("__devupProps=>((p)=>function(){return p.a;})(__devupProps)")]
#[case("__devupProps=>((p)=>p.a)(class {static a=2;})")]
#[case("__devupProps=>(async p=>p.a)(__devupProps)")]
#[case("async __devupProps=>((p)=>[p,p])(__devupProps.a)")]
#[case("__devupProps=>[ ((p)=>[(({a,...rest})=>rest)(p),p.a,p.b])(__devupProps) ]")]
#[case("__devupProps=>[ ((p)=>(({a,...rest})=>[rest,p.b])(p))(__devupProps) ]")]
#[case("__devupProps=>((p)=>{const a=p.a;return ((q)=>[a,q,q])(p.b);})(__devupProps)")]
#[case("__devupProps=>(([a,b])=>[a,b])([__devupProps.a,__devupProps.b])")]
#[case("__devupProps=>(({a,b})=>[a,b])(__devupProps)")]
#[case("__devupProps=>(({theme})=>[])(__devupProps)")]
#[case("__devupProps=>(()=>'red')(__devupProps.a)")]
#[case("__devupProps=>(([a,...rest])=>[a,rest])([__devupProps.a,__devupProps.b])")]
#[case("__devupProps=>(({a,...rest})=>[a,rest])(__devupProps)")]
#[case("__devupProps=>(({a:{b}})=>b)({'a':__devupProps})")]
#[case("__devupProps=>((p)=>{let q;[q]=[p.a];return [p.a,q];})(__devupProps)")]
#[case("__devupProps=>((p)=>{[p]=[2];return p;})(__devupProps)")]
#[case("__devupProps=>((p)=>({'a':p}={'a':2}))(__devupProps)")]
#[case("__devupProps=>((p)=>({a=2}=p))(__devupProps)")]
#[case("__devupProps=>(({a,...rest})=>()=>rest)(__devupProps)")]
#[case(
    "__devupProps=>(([p])=>p.a)({[Symbol.iterator]:function*(){try{trace.push('iterator');yield __devupProps;}finally{trace.push('close');}}})"
)]
#[case("__devupProps=>{'use strict';return ((p)=>[p,p])(__devupProps.a);}")]
#[case(
    "__devupProps=>[ ((p)=>{const record=['0'];return (record[0]='1',p.a)&&record;})(__devupProps) ]"
)]
#[case("__devupProps=>[ ((p)=>{const record=['0'];return ()=>record;})(__devupProps) ]")]
#[case("__devupProps=>(([,,])=>1)([__devupProps.a,__devupProps.b])")]
#[case("__devupProps=>(([,,...rest])=>rest)([__devupProps.a,__devupProps.b,7])")]
#[case("__devupProps=>((p)=>{trace.push(p.a);})(__devupProps)")]
#[case("__devupProps=>((p)=>{const record=['0'];return ()=>record;})(__devupProps)")]
#[case("__devupProps=>((p)=>([o.tag]=[p.a],[p.b,o.tag]))(__devupProps)")]
#[case("__devupProps=>((p)=>p())(o.method)")]
#[case("__devupProps=>((p)=>p('typeof __devupProps'))(eval)")]
#[case("__devupProps=>((p)=>{'use strict';return p.a;})(__devupProps)")]
#[case("__devupProps=>((p)=>{const {a}=p;return a;})(__devupProps)")]
#[case("__devupProps=>((p)=>{const record=['0'];return [record,p.a,<o.method/>];})(__devupProps)")]
#[case("__devupProps=>(({a})=>[a,<o.method/>])(__devupProps)")]
#[case("__devupProps=>(([p])=>{const value=p.a;return value;})([__devupProps])")]
fn renderer_values_effects_and_exceptions_match_the_original(#[case] source: &str) {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let output = compiled(&ast, parse(&ast, source), parse(&ast, "input()"));
    assert_eq!(
        execute(&output),
        execute(&format!("()=>({source})(input())")),
        "{output}"
    );
}

#[test]
fn compiler_created_component_tags_keep_their_binding_when_captured() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut component = parse(
        &ast,
        "__devupProps=>((Tag)=><Tag a={Tag.a}></Tag>)(Component)",
    );
    // Compiler-created tags also use JSXIdentifier for dynamic component names.
    rewrite_expression(&mut component, &mut |value| {
        if let Expression::JSXElement(element) = value {
            let JSXElement {
                opening_element,
                closing_element,
                ..
            } = &mut **element;
            for tag in std::iter::once(&mut opening_element.name)
                .chain(closing_element.iter_mut().map(|closing| &mut closing.name))
            {
                if let JSXElementName::IdentifierReference(id) = tag {
                    *tag = JSXElementName::new_identifier(
                        SPAN,
                        Str::from_in(id.name.as_str(), ast.allocator()),
                        &ast,
                    );
                }
            }
        }
        false
    });
    let original = readable_code(&component);
    let output = compiled(&ast, component, parse(&ast, "input()"));
    assert_eq!(output.matches("=>").count(), 1, "{output}");
    let observed = execute(&output);
    assert!(observed.contains("\"type\":\"Component\""), "{observed}");
    assert_eq!(observed, execute(&format!("()=>({original})(input())")));
}

#[test]
fn serialized_renderers_are_decoded_before_removing_their_intermediate_calls() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = "(__devupProps=>(({theme:omit,...dom})=>dom)(__devupProps))";
    let component = Expression::new_identifier(SPAN, Ident::from_in(source, ast.allocator()), &ast);
    let output = compiled(&ast, component, parse(&ast, "input()"));
    assert_eq!(output.matches("=>").count(), 1, "{output}");
    assert_eq!(
        execute(&output),
        execute(&format!("()=>({source})(input())"))
    );
}

#[test]
fn opaque_invalid_fragments_keep_their_original_call() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let component = Expression::new_identifier(SPAN, "invalid(", &ast);
    assert!(
        body(&ast, component, parse(&ast, "input()"))
            .as_expression()
            .is_some()
    );
}

#[rstest]
#[case("__devupProps=>[ ((p)=>[(({a,...rest})=>rest)(p),p.a,p.b])(__devupProps) ]")]
#[case("__devupProps=>(({theme})=>[])(__devupProps)")]
#[case("__devupProps=>(()=>'red')(__devupProps.a)")]
#[case("__devupProps=>(([a,...rest])=>[a,rest])([__devupProps.a,__devupProps.b])")]
#[case(
    "__devupProps=>[ ((p)=>{const record=['0'];return (record[0]='1',p.a)&&record;})(__devupProps) ]"
)]
fn eager_render_intermediates_use_only_the_existing_render_function(#[case] source: &str) {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let output = compiled(&ast, parse(&ast, source), parse(&ast, "input()"));
    assert_eq!(output.matches("=>").count(), 1, "{output}");
}
