use super::{
    AstBuilder, Base, CloneIn, Execution, Expression, ExtractStyleProp, GetAllocator, Naming,
    StyledContext, StyledDefinition, StyledRender, apply_attrs, clone_all, create_styled_component,
    gen_class_names, gen_styles, jsx_name, merge_expression_for_class_name, renders_tag, stages,
    unwrap_syntax_only, withheld,
};

/// Retarget a known styled definition without changing its execution owner.
pub fn with_component<'a>(
    ast_builder: &AstBuilder<'a>,
    definition: &StyledDefinition<'a>,
    target: &Expression<'a>,
    naming: Naming<'_>,
    context: StyledContext<'_>,
) -> Option<(Expression<'a>, StyledDefinition<'a>)> {
    let name = match unwrap_syntax_only(target) {
        Expression::StringLiteral(literal) => literal.value.to_string(),
        target => jsx_name(target)?,
    };
    let allocator = ast_builder.allocator();
    let base = Base {
        target: match unwrap_syntax_only(target) {
            Expression::StringLiteral(_) => None,
            _ => Some(target.clone_in(allocator)),
        },
        ..Base::named(name)
    };
    let mut scope = target.clone_in(allocator);
    let mut definition = definition.rebind(ast_builder, &mut scope);
    match &mut definition.execution {
        Execution::Staged {
            renderer, retarget, ..
        } => {
            renderer.native = renders_tag(&base.name, None);
            *retarget = Some(target.clone_in(allocator));
            definition.name.clone_from(&base.name);
            definition.target = base.target.as_ref().map(|value| value.clone_in(allocator));
            definition.capability = context.capability.cloned();
            let inherited = definition.clone_in(allocator);
            definition = definition.with_markers(naming.markers, Some(&inherited));
            let component = stages::create(
                ast_builder,
                &definition,
                stages::Context {
                    base: &base,
                    naming,
                    jsx: context.jsx,
                },
            )?;
            return Some((base.render(ast_builder, component), definition));
        }
        Execution::Delegated(renderer) => renderer.native = renders_tag(&base.name, None),
        Execution::Replay | Execution::RetainedFixed { .. } | Execution::RetainedDynamic { .. } => {
        }
    }
    let mut styles: Vec<ExtractStyleProp<'a>> = definition
        .styles
        .iter()
        .map(|style| style.clone_in(allocator))
        .collect();
    let classes: Vec<Expression<'a>> = clone_all(ast_builder, &definition.classes).collect();
    let class_name = merge_expression_for_class_name(
        ast_builder,
        clone_all(ast_builder, &classes).chain(gen_class_names(
            ast_builder,
            &mut styles,
            None,
            naming.split_filename,
        )),
    );
    let withheld = withheld(
        &definition.reads,
        renders_tag(&base.name, None),
        definition.forward.as_ref(),
    );
    let new_definition = base
        .definition(
            ast_builder,
            classes,
            &styles,
            &definition.attrs,
            (definition.reads.clone(), definition.forward.clone()),
            &definition.bindings,
        )
        .with_composition_styles(
            definition
                .composition_styles
                .as_ref()
                .map(|styles| styles.clone_in(allocator)),
        )
        .with_execution(ast_builder, Some(&definition))
        .with_css_slot(context.capability, Some(&definition))
        .with_markers(naming.markers, Some(&definition));
    let staged = stages::create(
        ast_builder,
        &new_definition,
        stages::Context {
            base: &base,
            naming,
            jsx: context.jsx,
        },
    );
    let has_stage = staged.is_some();
    let component = staged.unwrap_or_else(|| {
        create_styled_component(
            ast_builder,
            &base,
            StyledRender {
                class_name: &class_name,
                style_vars: &gen_styles(ast_builder, &styles, None),
                bindings: definition
                    .bindings
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone_in(allocator)))
                    .collect(),
                withheld: &withheld,
                capability: new_definition.capability.as_ref(),
                execution: &new_definition.execution,
                markers: new_definition.render_markers(ast_builder),
            },
            context.jsx,
        )
    });
    Some((
        new_definition.retain_bound(
            ast_builder,
            base.render(
                ast_builder,
                if has_stage {
                    component
                } else {
                    apply_attrs(ast_builder, component, &definition.attrs)
                },
            ),
        ),
        new_definition,
    ))
}
