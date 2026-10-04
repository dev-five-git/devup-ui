//! Execution markers for iterable operations Boa can fail without a call frame.

use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
};

use boa_engine::{Context, JsArgs, JsResult, JsString, NativeFunction};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, SourceType};

#[derive(Default)]
struct Active(RefCell<Vec<(u32, String, String)>>);

struct Arrays<'s> {
    source: &'s str,
    helper: &'s str,
    changes: Vec<(usize, String)>,
    sites: Vec<String>,
}

impl<'a> Visit<'a> for Arrays<'_> {
    fn visit_array_expression(&mut self, array: &oxc_ast::ast::ArrayExpression<'a>) {
        let mut endings = Vec::new();
        if array
            .elements
            .iter()
            .any(oxc_ast::ast::ArrayExpressionElement::is_spread)
        {
            let id = self.sites.len();
            self.sites.push(crate::locate(
                super::SCRIPT_PATH,
                self.source,
                array.span.start as usize,
            ));
            self.changes.push((
                array.span.start as usize,
                format!("({}({id}), {}({id}, ", self.helper, self.helper),
            ));
            endings.push((array.span.end as usize, "))".to_string()));
            for element in &array.elements {
                if let oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) = element {
                    let site = self.sites.len();
                    self.sites.push(crate::locate(
                        super::SCRIPT_PATH,
                        self.source,
                        spread.span.start as usize,
                    ));
                    let argument = spread.argument.span();
                    self.changes
                        .push((argument.start as usize, format!("{}({site}, ", self.helper)));
                    endings.push((argument.end as usize, ", true)".to_string()));
                }
            }
        }
        walk::walk_array_expression(self, array);
        self.changes.extend(endings);
    }
}

pub(crate) struct Operations {
    pub code: String,
    edits: Vec<crate::import_alias_visit::Edit>,
    source: String,
    helper: String,
    sites: Vec<String>,
}

impl Operations {
    pub(crate) fn new(source: &str) -> Self {
        let mut hash = rustc_hash::FxHasher::default();
        source.hash(&mut hash);
        let mut helper = format!("__devup_operation_site_{:x}__", hash.finish());
        while source.contains(&helper) {
            helper.push('_');
        }
        let allocator = oxc_allocator::Allocator::default();
        let parsed = oxc_parser::Parser::new(&allocator, source, SourceType::default()).parse();
        let mut arrays = Arrays {
            source,
            helper: &helper,
            changes: Vec::new(),
            sites: Vec::new(),
        };
        arrays.visit_program(&parsed.program);
        arrays.changes.sort_by_key(|(at, _)| *at);
        let mut code = String::new();
        let mut edits = Vec::new();
        let mut copied = 0;
        for (at, text) in arrays.changes {
            code.push_str(&source[copied..at]);
            code.push_str(&text);
            edits.push((at, at, text.len()));
            copied = at;
        }
        code.push_str(&source[copied..]);
        let sites = arrays.sites;
        Self {
            code,
            edits,
            source: source.to_string(),
            helper,
            sites,
        }
    }

    pub(crate) fn prepare(&self, context: &mut Context) -> JsResult<()> {
        context.insert_data(Active::default());
        context.register_global_builtin_callable(
            JsString::from(self.helper.as_str()),
            2,
            NativeFunction::from_copy_closure_with_captures(
                |_, args, sites, context| {
                    let id = args.get_or_undefined(0).to_u32(context)?;
                    let index = usize::try_from(id).map_err(|error| {
                        boa_engine::JsNativeError::range().with_message(error.to_string())
                    })?;
                    if let Some(active) = context.get_data::<Active>() {
                        let mut active = active.0.borrow_mut();
                        if args.len() == 1 {
                            if let Some(site) = sites.get(index) {
                                active.push((id, site.clone(), site.clone()));
                            }
                        } else if args.len() == 3 {
                            if let Some((_, _, place)) = active.last_mut()
                                && let Some(site) = sites.get(index)
                            {
                                site.clone_into(place);
                            }
                        } else if let Some(index) = active
                            .iter()
                            .rposition(|(operation, _, _)| *operation == id)
                        {
                            active.truncate(index);
                        }
                    }
                    Ok(args.get_or_undefined(1).clone())
                },
                self.sites.clone(),
            ),
        )
    }

    /// Rebase engine/read frames, adding a currently executing operation only
    /// when Boa supplied no script frame. A completed array has no active marker.
    pub(crate) fn explain(&self, error: &str, context: &Context) -> String {
        let marker = format!("({}:", super::SCRIPT_PATH);
        let active = context
            .get_data::<Active>()
            .and_then(|active| active.0.borrow().last().cloned());
        let mut first_frame = None;
        let mut explained = error
            .lines()
            .filter_map(|line| {
                let line = if line.contains("at <read> (") {
                    line.strip_suffix(" (native)").unwrap_or(line)
                } else {
                    line
                };
                let Some((prefix, position)) = line.rsplit_once(&marker) else {
                    return Some(line.to_string());
                };
                let Some((row, column)) = position.trim_end_matches(')').split_once(':') else {
                    return Some(line.to_string());
                };
                let (Ok(row), Ok(column)) = (row.parse::<u32>(), column.parse::<u32>()) else {
                    return Some(line.to_string());
                };
                let offset = crate::source_map::Lines::new(&self.code)
                    .code_point_offset(row.saturating_sub(1), column.saturating_sub(1));
                let mut inserted = 0;
                let generated = self.edits.iter().any(|(at, _, length)| {
                    let start = at + inserted;
                    inserted += length;
                    (start..start + length).contains(&offset)
                });
                let original = crate::import_alias_visit::source_offset(&self.edits, offset);
                let place = crate::locate(super::SCRIPT_PATH, &self.source, original);
                let operation =
                    generated || active.as_ref().is_some_and(|(_, array, _)| *array == place);
                first_frame.get_or_insert(operation);
                if operation && line.starts_with("    at ") {
                    return None;
                }
                Some(format!("{prefix}({place})"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        if first_frame.unwrap_or(true)
            && let Some((_, _, site)) = active
        {
            let at = explained.find("\n    at ").unwrap_or(explained.len());
            explained.insert_str(at, &format!("\n    at <operation> ({site})"));
        }
        explained
    }
}

#[cfg(test)]
mod coverage_tests;
