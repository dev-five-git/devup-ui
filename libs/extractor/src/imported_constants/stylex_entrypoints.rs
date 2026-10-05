use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::{Constant, ModuleScope, Modules, constant_literal};
use crate::extractor::KeyframesExtractResult;
use crate::extractor::extract_keyframes_from_expression::extract_keyframes_from_expression;
use crate::scope::stylex_bindings::{StylexBindings, symbol};
use crate::stylex::StylexFunction;
use crate::stylex::transitions::{
    DeclarationValue, IdentityDomain, TransitionApi, TransitionRules, content_name,
};
use crate::utils::unwrap_syntax_only;

impl ModuleScope<'_, '_> {
    pub(super) fn stylex_function(&self, callee: &Expression<'_>) -> Option<StylexFunction> {
        let scoping = self.semantic_scoping();
        self.stylex_bindings
            .get_or_init(|| StylexBindings::collect(self.program, scoping, &self.stylex_package))
            .function(callee, &|id| symbol(scoping, id))
    }

    pub(super) fn evaluate_stylex_rule(
        &mut self,
        modules: &mut Modules<'_>,
        function: StylexFunction,
        argument: &Expression<'_>,
    ) -> Option<Constant> {
        let value = self.evaluate(modules, argument)?;
        let api = match function {
            StylexFunction::Keyframes => return self.keyframes_constant(&value),
            StylexFunction::PositionTry => TransitionApi::Position,
            StylexFunction::ViewTransitionClass => TransitionApi::View,
            StylexFunction::Create
            | StylexFunction::Props
            | StylexFunction::Attrs
            | StylexFunction::DefineVars
            | StylexFunction::CreateTheme
            | StylexFunction::CreateThemeContract
            | StylexFunction::DefineConsts
            | StylexFunction::FirstThatWorks
            | StylexFunction::Include
            | StylexFunction::Types => return None,
        };
        let code = format!("{};", value.js_literal()?);
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &code, SourceType::ts()).parse();
        let Statement::ExpressionStatement(statement) = parsed.program.body.first()? else {
            return None;
        };
        let Expression::ObjectExpression(object) = unwrap_syntax_only(&statement.expression) else {
            return None;
        };
        let reader =
            |property: &str, expression: &Expression<'_>| match unwrap_syntax_only(expression) {
                Expression::NullLiteral(_) => Ok(DeclarationValue::Omitted),
                Expression::BooleanLiteral(boolean) if !boolean.value => {
                    Ok(DeclarationValue::Omitted)
                }
                Expression::BooleanLiteral(_) => Err(crate::utils::build_time_error(
                    api.name(),
                    "true",
                    "use a static string/number or omitted null/undefined/false",
                )),
                Expression::Identifier(identifier) if identifier.name == "undefined" => {
                    Ok(DeclarationValue::Omitted)
                }
                expression => crate::stylex::stylex_value(property, expression)
                    .map(|value| DeclarationValue::Scalar(value.into_owned()))
                    .ok_or_else(|| {
                        crate::utils::runtime_value_error(
                            api.name(),
                            &crate::utils::readable_code(expression),
                        )
                    }),
            };
        let rules = match TransitionRules::parse(api, object, &reader) {
            Ok(rules) => rules,
            Err(_) => return None,
        };
        let (name, _) = rules.compile(self.path);
        Some(Constant::String(name))
    }

    fn keyframes_constant(&self, value: &Constant) -> Option<Constant> {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let mut expression = constant_literal(&builder, value, true)?;
        if !matches!(expression, Expression::ObjectExpression(_)) {
            return None;
        }
        let KeyframesExtractResult {
            keyframes,
            runtime_value,
        } = extract_keyframes_from_expression(&builder, &mut expression);
        if runtime_value.is_some() {
            return None;
        }
        let mut content = String::new();
        for (step, styles) in &keyframes.keyframes {
            content.push_str(step);
            content.push('{');
            for style in styles {
                content.push_str(style.property());
                content.push(':');
                content.push_str(style.value());
                content.push(';');
            }
            content.push('}');
        }
        Some(Constant::String(content_name(
            self.path,
            IdentityDomain::Keyframes,
            &content,
        )))
    }
}
