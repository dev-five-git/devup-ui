use super::{
    AstBuilder, Base, CloneIn, Evaluations, Execution, Expression, FromIn, GetAllocator, Naming,
    SPAN, StyledDefinition, StyledJsx, gen_class_names, gen_styles,
    merge_expression_for_class_name, residual, sets_plain_props, withheld, without_props,
};
use crate::utils::readable_code as code;
use oxc_ast::ast::{Ident, Program};
use oxc_span::SourceType;
#[path = "styled_stage_render.rs"]
mod render;
use render::{stage_attrs, stage_element};

pub(super) struct Context<'b, 'a, 's> {
    pub base: &'b Base<'a>,
    pub naming: Naming<'s>,
    pub jsx: &'b mut Option<StyledJsx>,
}

pub(super) fn expression<'a>(ast: &AstBuilder<'a>, code: &str) -> Expression<'a> {
    Expression::new_identifier(
        SPAN,
        Ident::from_in(format!("({code})").as_str(), ast.allocator()),
        ast,
    )
}

pub(super) fn create<'a>(
    ast: &AstBuilder<'a>,
    definition: &StyledDefinition<'a>,
    context: Context<'_, 'a, '_>,
) -> Option<Expression<'a>> {
    let capability = definition.capability.as_ref()?;
    if definition.fixed_summary() || definition.portable() {
        return None;
    }
    let allocator = ast.allocator();
    let own = residual::selections(ast, definition.composition_styles(), context.naming);
    let mut reservation = definition.code().collect::<Vec<_>>().join(";");
    for (name, value) in &definition.bindings {
        reservation.push_str(name);
        reservation.push_str(&code(value));
    }
    for attr in &definition.attrs {
        reservation.push_str(&code(attr));
    }
    for value in context.base.target.iter().chain(&context.base.bound) {
        reservation.push_str(&code(value));
    }
    match &definition.execution {
        Execution::Staged {
            renderer, retarget, ..
        } => {
            reservation.push_str(&code(&renderer.target));
            if let Some(value) = retarget {
                reservation.push_str(&code(value));
            }
        }
        Execution::Replay
        | Execution::RetainedFixed { .. }
        | Execution::RetainedDynamic { .. }
        | Execution::Delegated(_) => {}
    }
    let mut names = Evaluations::new(ast, reservation);
    let mut fresh = || names.fresh_name();
    let input = fresh();
    let control = fresh();
    let clean = fresh();
    let props = fresh();
    let frame = fresh();
    let values = fresh();
    let result = fresh();
    let start = fresh();
    let attrs = fresh();
    let render = fresh();
    let caller = fresh();
    let dropped = fresh();
    let reserved = fresh();
    let hybrid = fresh();
    let own_default = fresh();
    let resolver = fresh();
    let resolved = fresh();
    let input_as = fresh();
    let plain = definition.attrs.iter().all(sets_plain_props);
    let hybrid_value = if plain {
        format!("{control}?{control}[2]:false")
    } else {
        "true".to_string()
    };
    let slot = capability.classes.name();
    let forwarded_slot = capability.forwarded.name();
    let own_target = match &definition.execution {
        Execution::Staged { retarget, .. } => retarget.as_ref().map_or_else(
            || "(void 0)".to_string(),
            |target| format!("()=>({})", code(target)),
        ),
        Execution::Replay
        | Execution::RetainedFixed { .. }
        | Execution::RetainedDynamic { .. }
        | Execution::Delegated(_) => "(void 0)".to_string(),
    };
    let resolve_target = format!(
        "const {own_default}={own_target};const {resolver}=({control}&&{control}[3])??{own_default};"
    );
    let class_values = own
        .values
        .iter()
        .map(|value| value.as_ref().map_or_else(|| "\"\"".to_string(), code))
        .map(|value| format!("({value})"))
        .collect::<Vec<_>>()
        .join(",");
    let style_vars = gen_styles(ast, &definition.styles, None);
    let variables = style_vars
        .as_ref()
        .map_or_else(|| "(void 0)".to_string(), code);
    let marker_value = definition.render_markers(ast);
    let has_markers = marker_value.is_some() || !definition.classes.is_empty();
    let markers = marker_value
        .as_ref()
        .map_or_else(|| "(void 0)".to_string(), code);
    let opaque = definition
        .classes
        .iter()
        .map(|value| format!("({})", code(value)))
        .collect::<Vec<_>>()
        .join(",");
    let statements = super::super::styled_record_evaluation::capture_statements(
        ast,
        definition
            .bindings
            .iter()
            .map(|(name, value)| (name.clone(), value.clone_in(allocator)))
            .collect(),
    );
    let program = Program::new(
        SPAN,
        SourceType::d_ts(),
        "",
        oxc_allocator::Vec::new_in(ast),
        None,
        oxc_allocator::Vec::new_in(ast),
        statements,
        ast,
    );
    let bindings = oxc_codegen::Codegen::new().build(&program).code;
    let withheld = withheld(
        &definition.reads,
        definition.native(),
        definition.forward.as_ref(),
    );
    let forwarded = super::super::styled_transient::filter(ast, without_props(ast, &withheld));
    let finish = format!(
        "{result}[4]({{...{},as:{result}[3],forwardedAs:{props}.forwardedAs,className:({result}[0].length||{result}[2]!==void 0)?[...{result}[0],{result}[2],{props}.className].filter(Boolean).join(' '):{props}.className,style:{result}[1]===void 0?{props}.style:{{...{result}[1],...{props}.style}}}})",
        code(&forwarded),
    );
    let dispatch = format!("{control}?(0,{control}[1])({props},{result}):{finish}");
    let merged_attrs = stage_attrs(
        ast,
        &definition.attrs,
        render::Attrs {
            input: &clean,
            hybrid: &hybrid,
            plain,
        },
    );
    let code = match &definition.execution {
        Execution::Staged {
            renderer, shape, ..
        } => {
            let keys = shape.merge(&own.shape, &frame, &values);
            let variables = if style_vars.is_some() {
                format!("{{...{frame}[1],...{variables}}}")
            } else {
                format!("{frame}[1]")
            };
            let markers = if has_markers {
                format!("[{frame}[2],{markers},{opaque}].filter(Boolean).join(' ')")
            } else {
                format!("{frame}[2]")
            };
            let body = format!(
                "({props},{frame})=>{{const {{style,className,as:DevupAs={frame}[3],forwardedAs}}={props};const rest={frame}[5];{bindings}const {values}=[{class_values}];const {result}=[{keys},{variables},{markers},{frame}[3],{frame}[4],rest];return {dispatch};}}"
            );
            let next = format!(
                "({clean},{start},{hybrid})=>{{const {attrs}={};return {control}?(0,{control}[0])({attrs},{start},{hybrid}):{start}({attrs});}}",
                code(&merged_attrs)
            );
            let incoming = format!(
                "{{...{clean},{}:[{attrs},{render},{hybrid},{resolver}]}}",
                renderer.capability.classes.name()
            );
            let target = renderer.target.clone_in(allocator);
            let element = stage_element(ast, (target, expression(ast, &incoming)), context.jsx);
            format!(
                "{input}=>{{const {{{slot}:{control},{forwarded_slot}:{reserved},...{clean}}}={input};{resolve_target}const {hybrid}={hybrid_value};const {attrs}={next};const {render}={body};return {};}}",
                code(&element)
            )
        }
        Execution::Replay => {
            let mut ordinary = definition
                .styles
                .iter()
                .map(|value| value.clone_in(allocator))
                .collect::<Vec<_>>();
            let ordinary = merge_expression_for_class_name(
                ast,
                gen_class_names(ast, &mut ordinary, None, context.naming.split_filename),
            );
            let ordinary = ordinary
                .as_ref()
                .map_or_else(|| "[]".to_string(), |value| format!("[{}]", code(value)));
            let markers = if has_markers {
                format!("[{markers},{opaque}].filter(Boolean).join(' ')")
            } else {
                "(void 0)".to_string()
            };
            let terminal = render::terminal(ast, (context.base, &resolved), context.jsx);
            let target = context
                .base
                .target
                .as_ref()
                .map_or_else(|| format!("{:?}", context.base.name), code);
            let style_body = format!(
                "({props})=>{{const {{style,className,as:{input_as},forwardedAs,...rest}}={props};let DevupAs={input_as};if(DevupAs===void 0){{if({resolver}){{DevupAs = (0,{resolver})();}}else{{DevupAs = {target};}}}}const {resolved}=DevupAs;{bindings}const {result}=[{control}?[{class_values}]:{ordinary},{variables},{markers},{resolved},{},rest];return {dispatch};}}",
                code(&terminal)
            );
            format!(
                "{input}=>{{const {{{slot}:{control},{forwarded_slot}:{reserved},...{dropped}}}={input};{resolve_target}const {hybrid}={hybrid_value};const {caller}={dropped}.className;const {clean}={{...{dropped},className:void 0}};const {render}={style_body};const {start}={props}=>{render}({{...{props},className:{hybrid}?([{props}.className,{caller}].filter(Boolean).join(' ')||void 0):{caller}}});const {attrs}={};return {control}?(0,{control}[0])({attrs},{start},{hybrid}):{start}({attrs});}}",
                code(&merged_attrs)
            )
        }
        Execution::RetainedFixed { .. }
        | Execution::RetainedDynamic { .. }
        | Execution::Delegated(_) => return None,
    };
    Some(expression(ast, &code))
}
