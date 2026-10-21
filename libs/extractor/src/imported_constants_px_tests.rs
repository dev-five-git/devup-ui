use super::*;
use rstest::rstest;

#[test]
fn own_css_record_keeps_fallback_order_when_authored_px_mark_is_inlined() {
    // Given: Emotion-style fallback arrays, not responsive breakpoints.
    let source = "import { css } from '@devup-ui/react';
const rules = { padding: [2, 4], opacity: [0, 0.5] };
export const card = css`${rules}`;";
    let allocator = Allocator::default();
    let mut program = crate::parsing::parse(&allocator, "/src/fallback.ts", source)
        .unwrap_or_else(|error| panic!("invalid fallback fixture: {error}"));
    let mut marks = crate::import_alias_visit::PxMarks::default();
    let start = u32::try_from(
        source
            .rfind("rules}")
            .unwrap_or_else(|| panic!("missing interpolation")),
    )
    .unwrap_or_else(|error| panic!("fixture offset exceeds u32: {error}"));
    marks.insert((start, start + 5));
    // When
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "/src/fallback.ts",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &marks,
    );
    // Then: preserve ordered fallbacks, adding px only to length values.
    assert_eq!(inlined.css_failures.len(), 0);
    let Some(Statement::ExportDeclaration(export)) = program.body.last() else {
        panic!("expected card")
    };
    let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration else {
        panic!("expected declaration")
    };
    let Some(Expression::TaggedTemplateExpression(tagged)) =
        declaration.declarations[0].init.as_ref()
    else {
        panic!("expected tag")
    };
    let Expression::ObjectExpression(object) = &tagged.quasi.expressions[0] else {
        panic!("expected materialized rules")
    };
    let values: Vec<_> = object
        .properties
        .iter()
        .map(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                panic!("expected property")
            };
            let Expression::StringLiteral(value) = &property.value else {
                panic!("expected ordered fallback")
            };
            (
                property
                    .key
                    .static_name()
                    .unwrap_or_else(|| panic!("expected static key"))
                    .to_string(),
                value.value.to_string(),
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            ("padding".to_string(), "2px;padding:4px".to_string()),
            ("opacity".to_string(), "0;opacity:0.5".to_string())
        ]
    );
}

#[rstest]
fn own_css_record_honors_authored_px_marks_when_inlined(#[values(false, true)] marked: bool) {
    // Given: only the authored interpolation is marked, not its declaration.
    let source = "import { css } from '@devup-ui/react';
const rules = { padding: 2, p: 3, opacity: 0.5, margin: 0, vars: { '--gap': 7 } };
export const card = css`${rules}`;";
    let allocator = Allocator::default();
    let mut program = crate::parsing::parse(&allocator, "/src/px.ts", source)
        .unwrap_or_else(|error| panic!("invalid px fixture: {error}"));
    let mut marks = crate::import_alias_visit::PxMarks::default();
    let start = u32::try_from(
        source
            .rfind("rules}")
            .unwrap_or_else(|| panic!("missing authored interpolation")),
    )
    .unwrap_or_else(|error| panic!("fixture offset exceeds u32: {error}"));
    if marked {
        marks.insert((start, start + 5));
    }

    // When: run the lower-level contract directly, without an alias transform.
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "/src/px.ts",
        &ExtractOption::default(),
        None,
        CssProp::Off,
        &marks,
    );

    // Then: marked lengths become px, but unitless, zero and vars stay numeric.
    assert_eq!(inlined.css_failures.len(), 0);
    let Some(Statement::ExportDeclaration(export)) = program.body.last() else {
        panic!("expected exported card");
    };
    let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration else {
        panic!("expected card declaration");
    };
    let Some(Expression::TaggedTemplateExpression(tagged)) =
        declaration.declarations[0].init.as_ref()
    else {
        panic!("expected own css tag");
    };
    let Expression::ObjectExpression(object) = &tagged.quasi.expressions[0] else {
        panic!("expected materialized record");
    };
    for (property, (key, number)) in object.properties.iter().zip([
        ("padding", 2.0),
        ("p", 3.0),
        ("opacity", 0.5),
        ("margin", 0.0),
        ("vars", 7.0),
    ]) {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            panic!("expected record property");
        };
        assert_eq!(property.key.static_name().as_deref(), Some(key));
        let value = if key == "vars" {
            let Expression::ObjectExpression(vars) = &property.value else {
                panic!("expected vars record");
            };
            let ObjectPropertyKind::ObjectProperty(variable) = &vars.properties[0] else {
                panic!("expected custom property");
            };
            &variable.value
        } else {
            &property.value
        };
        if marked && key == "padding" {
            let Expression::StringLiteral(value) = value else {
                panic!("expected marked length in px, got {value:?}");
            };
            assert_eq!(value.value, "2px");
        } else {
            assert_eq!(crate::utils::js_number_literal(value), Some(number));
        }
    }
    assert_eq!(object.properties.len(), 5);
}
