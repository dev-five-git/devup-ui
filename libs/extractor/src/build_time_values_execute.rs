//! Execution of the exact value closures selected by the lexical reader.

use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use std::{collections::BTreeSet, path::Path};

use super::mapping::Mapped;
use super::{Located, Replacement, definitions, find, parse, value_script};
use crate::evaluation_sandbox::{Failure, Sandbox};
use crate::{ExtractOption, ModuleResolver};

const HELPERS: &str = r#"const __failed__ = (() => { const fail = () => { throw new ReferenceError("its value threw"); }; return new Proxy(function () {}, { get: fail, apply: fail, construct: fail, getPrototypeOf: fail }); })();
const __failed_statements__ = new Set();
const __try__ = (compute, statement) => { try { return compute(); } catch { __failed_statements__.add(statement); return __failed__; } };
const __literal__ = (value) => {
  const plain = (item) => item === undefined || item === null || typeof item === "string" || typeof item === "boolean" || (typeof item === "number" && Number.isFinite(item))
    || (Array.isArray(item) && item.every(plain))
    || (typeof item === "object" && Object.getPrototypeOf(item) === Object.prototype && Object.getOwnPropertySymbols(item).length === 0 && !Object.prototype.hasOwnProperty.call(item, "__proto__") && Object.values(item).every(plain));
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : undefined;
  return value !== undefined && plain(value) ? JSON.stringify(value) : undefined;
};
"#;

const HELPERS_SOURCE: &str = "<devup-build-time-helpers>";
const DEFINITIONS_SOURCE: &str = "<devup-build-time-definitions>";
const VALUE_SOURCE: &str = "<devup-build-time-value>";
type Computed = (Vec<Replacement>, BTreeSet<String>);

fn stop(
    failure: Failure,
    scripts: &[Mapped],
    code: &str,
) -> Result<Option<Computed>, Vec<Located>> {
    let violations = match failure {
        Failure::Forbidden(violations) => violations,
        Failure::Js(_) => return Ok(None),
    };
    let mut errors: Vec<Located> = violations
        .iter()
        .map(|violation| {
            let offset = scripts.iter().find_map(|script| script.locate(violation));
            let Some(offset) = offset else {
                return (0, violation.error().to_string());
            };
            let tail = &code[offset..];
            let read = tail
                .split_once('\n')
                .map_or(tail, |(line, _)| line)
                .trim_end_matches(';');
            (
                offset,
                format!(
                    "`{}` cannot use `{}` at build time: {}",
                    violation.name(),
                    read,
                    violation.requirement()
                ),
            )
        })
        .collect();
    errors.dedup();
    Err(errors)
}

pub(super) fn compute(
    code: &str,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    unknown: &crate::imported_constants::Unknown,
) -> Result<Option<Computed>, Vec<Located>> {
    let allocator = Allocator::default();
    let Some(program) = parse(&allocator, filename, code) else {
        return Ok(None);
    };
    let is_style = |source: &str| {
        source.starts_with(option.package.as_str()) || option.import_aliases.contains_key(source)
    };
    let changes = crate::imported_constants::ChangeCheck::new(&program, filename, option, resolver);
    let found = find(
        &program,
        &is_style,
        unknown,
        &|name| changes.is_changed(name),
        &|name| changes.known(name),
    );
    if found.is_empty() {
        return Ok(None);
    }
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| vec![(0, error.to_string())])?;
    if let Err(failure) = sandbox.run_source(
        &mut context,
        Source::from_bytes(HELPERS).with_path(Path::new(HELPERS_SOURCE)),
    ) {
        return stop(failure, &[], code);
    }
    let module = definitions(&program, code, &found);
    let mut scripts = vec![Mapped::new(module, filename, DEFINITIONS_SOURCE)];
    sandbox
        .prepare(&mut context, &scripts[0].instrumented)
        .map_err(|error| vec![(0, error.to_string())])?;
    if let Err(failure) = sandbox.run_source(
        &mut context,
        Source::from_bytes(&scripts[0].instrumented.code).with_path(Path::new(DEFINITIONS_SOURCE)),
    ) {
        return stop(failure, &scripts, code);
    }
    let mut computed = Vec::new();
    for found in found {
        let value_source = format!("{VALUE_SOURCE}:{}", found.span.start);
        let script = Mapped::new(value_script(code, &found), filename, &value_source);
        sandbox
            .prepare(&mut context, &script.instrumented)
            .map_err(|error| vec![(0, error.to_string())])?;
        let result = sandbox.run_source(
            &mut context,
            Source::from_bytes(&script.instrumented.code).with_path(Path::new(&value_source)),
        );
        scripts.push(script);
        let value = match result {
            Ok(value) => value,
            Err(failure) => return stop(failure, &scripts, code),
        };
        let Some(literal) = value
            .as_string()
            .map(|literal| literal.to_std_string_escaped())
        else {
            continue;
        };
        if found.rules_only && !literal.starts_with(['{', '[', '"']) {
            continue;
        }
        computed.push((
            found.span,
            match found.shorthand {
                Some(key) => format!("{key}: {literal}"),
                None => literal,
            },
        ));
    }
    Ok((!computed.is_empty()).then_some((computed, changes.dependencies())))
}

#[cfg(test)]
mod build_time_values_execute_coverage_tests;
