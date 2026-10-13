pub(super) use super::cache6_test_support::*;
pub(super) use super::counter_compiler::{after_finish, code_extract_counter};
pub(super) use extractor::compiler_policy::{
    CompilerInput, CounterCompileError, with_counter_extract,
};
pub(super) use std::panic::{AssertUnwindSafe, catch_unwind};

#[cfg(test)]
mod cold;
#[cfg(test)]
mod dynamic;
#[cfg(test)]
mod nested;
#[cfg(test)]
mod surfaces;

pub(super) fn required<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("{error:?}"))
}

pub(super) fn input<'a>(file: &'a str, code: &'a str) -> CompilerInput<'a> {
    CompilerInput {
        filename: file,
        code,
        option: ExtractOption::default(),
        source_map: true,
        resolver: None,
    }
}
pub(super) fn compile(file: &str, code: &str) -> ObservedOutput {
    let output = code_extract_counter(input(file, code), false).unwrap_or_else(|e| panic!("{e}"));
    ObservedOutput::capture(&output, file)
}
pub(super) const PARENT: &str = "import {tone} from './child.css'; import {style} from '@devup-ui/react'; export const root=style({color:tone});";
pub(super) const RESERVED_CHILD: &ModuleResolver = &|_, _| {
    Some(ResolvedModule { path: "child.css.ts".into(), code: "import {style} from '@devup-ui/react'; export const tone='red'; export const unused=style({color:'magenta'});".into() })
};
pub(super) fn reserved_input() -> CompilerInput<'static> {
    let mut source = input("parent.css.ts", PARENT);
    source.resolver = Some(RESERVED_CHILD);
    source
}
