use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case::catch_outside("(...all)=>{try{}catch(all){}return all[0].theme;}", true)]
#[case::catch_inside("(...all)=>{try{}catch(all){return all[0].theme;}}", false)]
#[case::catch_pattern("(...all)=>{try{}catch({all}){return all[0].theme;}}", false)]
#[case::catch_default_outer("(...all)=>{try{}catch({x=all[0].theme}){}}", true)]
#[case::catch_body_lexical("(...all)=>{try{}catch({x=all[0].theme}){let all;}}", true)]
#[case::arguments_catch_outside(
    "function(){try{}catch(arguments){}return arguments[0].theme;}",
    true
)]
#[case::arguments_catch_inside(
    "function(){try{}catch(arguments){return arguments[0].theme;}}",
    false
)]
#[case::for_outside("(...all)=>{for(let all=0;false;){}return all[0].theme;}", true)]
#[case::for_inside("(...all)=>{for(let all=[];all[0].theme;all[0].theme){}}", false)]
#[case::for_initializer("(...all)=>{for(let all=all[0].theme;false;){}}", false)]
#[case::for_body_lexical("(...all)=>{for(;all[0].theme;){let all;}}", true)]
#[case::for_in_outside("(...all)=>{for(const all in {}){}return all[0].theme;}", true)]
#[case::for_in_inside("(...all)=>{for(const all in {}){all[0].theme;}}", false)]
#[case::for_in_rhs("(...all)=>{for(const all in all[0].theme){}}", false)]
#[case::for_of_outside("(...all)=>{for(const all of []){}return all[0].theme;}", true)]
#[case::for_of_inside("(...all)=>{for(const all of []){all[0].theme;}}", false)]
#[case::for_of_rhs("(...all)=>{for(const all of all[0].theme){}}", false)]
#[case::arguments_for_outside(
    "function(){for(let arguments;false;){}return arguments[0].theme;}",
    true
)]
#[case::arguments_for_in_inside("function(){for(let arguments in {}){arguments[0].theme;}}", false)]
#[case::arguments_for_of_outside(
    "function(){for(let arguments of []){}return arguments[0].theme;}",
    true
)]
#[case::switch_outside("(...all)=>{switch(0){case 0:let all;}return all[0].theme;}", true)]
#[case::switch_discriminant("(...all)=>{switch(all[0].theme){case 0:let all;}}", true)]
#[case::switch_shared_cases(
    "(...all)=>{switch(0){case all[0].theme:break;default:let all;}}",
    false
)]
#[case::switch_nested_block("(...all)=>{switch(0){case all[0].theme:{let all;}}}", true)]
#[case::arguments_switch_outside(
    "function(){switch(0){case 0:let arguments;}return arguments[0].theme;}",
    true
)]
#[case::block_lexical_outside("(...all)=>{{let all;}return all[0].theme;}", true)]
#[case::block_lexical_inside("(...all)=>{{all[0].theme;let all;}}", false)]
#[case::block_function_outside("(...all)=>{{function all(){}}return all[0].theme;}", true)]
#[case::block_function_inside("(...all)=>{{function all(){}all[0].theme;}}", false)]
#[case::block_class_inside("(...all)=>{{class all{}all[0].theme;}}", false)]
#[case::root_rest_var("(...all)=>{var all;return all[0].theme;}", true)]
#[case::root_rest_nested_var("(...all)=>{{var all;}return all[0].theme;}", true)]
#[case::root_arguments_var("function(){var arguments;return arguments[0].theme;}", true)]
#[case::root_arguments_nested_var(
    "function(){if(false){var arguments;}return arguments[0].theme;}",
    true
)]
#[case::root_arguments_default_var("function(p=0){var arguments;return arguments[0].theme;}", true)]
#[case::root_named_arguments("function arguments(){return arguments[0].theme;}", true)]
#[case::root_rest_named_arguments(
    "(...arguments)=>{var arguments;return arguments[0].theme;}",
    true
)]
#[case::root_arguments_parameter("function(arguments){return arguments[0].theme;}", false)]
#[case::root_arguments_function(
    "function(){function arguments(){}return arguments[0].theme;}",
    false
)]
#[case::root_rest_function("(...all)=>{function all(){}return all[0].theme;}", false)]
#[case::nested_block_var("(...all)=>(()=>{all[0].theme;{var all;}})()", false)]
#[case::nested_if_var("(...all)=>(()=>{if(false){var all;}return all[0].theme;})()", false)]
#[case::nested_catch_var(
    "(...all)=>(()=>{try{}catch(e){var all;}return all[0].theme;})()",
    false
)]
#[case::nested_for_var("(...all)=>(()=>{for(var all;false;){}return all[0].theme;})()", false)]
#[case::nested_for_in_var("(...all)=>(()=>{for(var all in {}){}return all[0].theme;})()", false)]
#[case::nested_for_of_var("(...all)=>(()=>{for(var all of []){}return all[0].theme;})()", false)]
#[case::nested_switch_var(
    "(...all)=>(()=>{switch(0){case 0:var all;}return all[0].theme;})()",
    false
)]
#[case::nested_normal_rest_var(
    "(...all)=>function(){if(false){var all;}return all[0].theme;}",
    false
)]
#[case::nested_normal_arguments(
    "function(){return function(){return arguments[0].theme;};}",
    false
)]
#[case::nested_arrow_arguments("function(){return ()=>arguments[0].theme;}", true)]
#[case::nested_arrow_arguments_var(
    "function(){return ()=>{if(false){var arguments;}return arguments[0].theme;};}",
    false
)]
#[case::nested_arrow_param("(...all)=>(all)=>all[0].theme", false)]
#[case::nested_normal_name("(...all)=>function all(){return all[0].theme;}", false)]
#[case::default_before_body_var("(...all)=>(x=all[0].theme)=>{var all;}", true)]
#[case::default_before_body_let("(...all)=>(x=all[0].theme)=>{let all;}", true)]
#[case::root_default_before_let("function(x=arguments[0].theme){let arguments;}", true)]
#[case::parameter_order("function(p,x=arguments[0].theme){return x;}", true)]
#[case::later_parameter_tdz("(...all)=>(x=all[0].theme,all)=>x", false)]
#[case::destructured_parameter_default("(...all)=>({x=all[0].theme})=>x", true)]
#[case::inner_function_var_boundary(
    "(...all)=>(()=>{function local(){var all;}return all[0].theme;})()",
    true
)]
#[case::inner_arrow_var_boundary(
    "(...all)=>(()=>{const local=()=>{var all;};return all[0].theme;})()",
    true
)]
#[case::inner_class_var_boundary(
    "(...all)=>(()=>{class Local{static{var all;}}return all[0].theme;})()",
    true
)]
#[case::class_expression_name("(...all)=>(class all{static x=all[0].theme;})", false)]
#[case::class_expression_outer("(...all)=>(class Local{static x=all[0].theme;})", true)]
#[case::static_block_var("(...all)=>(class Local{static{all[0].theme;{var all;}}})", false)]
#[case::static_block_restore(
    "(...all)=>(class Local{static{var all;}static x=all[0].theme;})",
    true
)]
fn implicit_theme_detection_follows_binding_owners(#[case] callback: &str, #[case] expected: bool) {
    // Given: a parsed callback with a distinct lexical or function binding owner.
    let arena = Allocator::default();
    let source = format!("({callback});");
    let parsed = Parser::new(&arena, &source, SourceType::script()).parse();
    assert_eq!(
        parsed.diagnostics.len(),
        0,
        "{callback}: {:?}",
        parsed.diagnostics
    );
    assert!(!parsed.fatal_error, "{callback}");
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expected callback expression: {callback}");
    };
    // When: the same detector used by the located guard resolves implicit props roots.
    let observed = super::reads(&statement.expression);
    // Then: only the actual callback roots are recognized, without invoking Boa.
    assert_eq!(observed, expected, "{callback}");
}
