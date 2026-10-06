use oxc_ast::ast::VariableDeclarationKind;

use super::View;
use crate::module_loader::Mapped;
use crate::ordinary_ve::selection::plan::{Binding, UnitKind};
use crate::vanilla_extract::{Stylesheet, json_string};

pub(crate) struct Rendered {
    pub mapped: Mapped,
    pub bindings: Vec<Binding>,
    pub native: Vec<Binding>,
}

impl View {
    pub(crate) fn render(
        &self,
        stylesheet: Stylesheet<'_>,
        option: &crate::ExtractOption,
    ) -> Result<Rendered, String> {
        let mut mapped = Mapped::default();
        crate::ordinary_ve::execution::imports::write(
            crate::ordinary_ve::execution::SelectedModule {
                stylesheet,
                selection: &self.selection,
            },
            option,
            &mut mapped,
        )?;
        for span in &self.reexports {
            mapped.copy(stylesheet.code, *span);
            mapped.synthesize(span.end, "\n");
        }
        for unit in &self.selection.units {
            if let Some((span, name)) = &self.default
                && *span == unit.span
            {
                mapped.synthesize(span.start, &format!("const {name}=("));
            }
            match unit.kind {
                UnitKind::Declarator { kind, erased, .. } => {
                    if erased {
                        return Err(format!(
                            "{}: required input is erased by TypeScript. Fix: provide a runtime value",
                            crate::locate(
                                stylesheet.filename,
                                stylesheet.source,
                                usize::try_from(unit.span.start).unwrap_or(0)
                            )
                        ));
                    }
                    let keyword = match kind {
                        VariableDeclarationKind::Const => "const",
                        VariableDeclarationKind::Let => "let",
                        VariableDeclarationKind::Var => "var",
                        VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => {
                            return Err(format!(
                                "{}:1:1: required resource declaration cannot run exactly. Fix: use a plain initializer",
                                stylesheet.filename
                            ));
                        }
                    };
                    mapped.synthesize(unit.span.start, &format!("{keyword} "));
                }
                UnitKind::Function { erased: true } => {
                    return Err(format!(
                        "{}:1:1: required callable is erased by TypeScript. Fix: provide a runtime function",
                        stylesheet.filename
                    ));
                }
                UnitKind::Function { erased: false } | UnitKind::Statement => {}
            }
            match self.properties.get(&unit.node) {
                Some(properties) => properties.replace(stylesheet.code, unit.span, &mut mapped),
                None => mapped.copy(stylesheet.code, unit.span),
            }
            if self
                .default
                .as_ref()
                .is_some_and(|(span, _)| *span == unit.span)
            {
                mapped.synthesize(unit.span.end, ");");
            }
            mapped.synthesize(unit.span.end, ";\n");
        }
        for (exported, local) in &self.exports {
            mapped.synthesize(
                0,
                &format!("export {{{local} as {}}};\n", json_string(exported)),
            );
        }
        let bindings = self
            .selection
            .units
            .iter()
            .flat_map(|unit| &unit.bindings)
            .cloned()
            .collect();
        let native = self
            .selection
            .roots
            .iter()
            .flat_map(|root| &root.captures)
            .cloned()
            .collect();
        Ok(Rendered {
            mapped,
            bindings,
            native,
        })
    }
}
