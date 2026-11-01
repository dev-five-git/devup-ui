pub const CALLS: [&str; 34] = [
    "css(yellow, azure)",
    "css(azure, yellow)",
    "css({ color: 'green' }, yellow)",
    "css(yellow, { color: 'green' })",
    "css([yellow, azure])",
    "css(...[yellow, azure])",
    "css(yellow, null, undefined, false, azure)",
    "css(yellow, on && azure)",
    "css(on ? yellow : azure)",
    "css(yellow, on ? { color: 'pink' } : null)",
    "css(yellow, { color: on ? 'pink' : 'teal' })",
    "css(yellow, ext, azure)",
    "css(yellow, on ? 'plain' : azure)",
    "css(azure, wide)",
    "css(wide, { color: [null, 'x2'] })",
    "css(yellow, ordered)",
    "css(heading, body)",
    "css(chained, { color: 'gold' })",
    "css(yellow, { color: { a: 'red', b: 'blue' }[size] })",
    "css(yellow, on ? { color: { a: 'red' }[size] } : null)",
    "css(yellow, on || azure)",
    "css(yellow, ext ?? azure)",
    "css(yellow, null ?? azure)",
    "css(yellow, false ?? azure)",
    "css(azure || yellow)",
    "css(yellow, on || 'plain')",
    "css(yellow, { color: on ? 'a' : 'b', m: [1, 2], styleOrder: 2 })",
    "css(yellow, { color: { a: 'x' }[size], styleOrder: 2 })",
    "css(yellow, on ? { color: other ? 'a' : 'b' } : null)",
    "css(yellow, fade)",
    "css(yellow, { color: ['x', 'y'][idx], styleOrder: 2 })",
    "css(yellow, { color: 'pink' } || azure)",
    "css(yellow, undefined ?? azure)",
    "css(yellow, false ?? azure)",
];

pub fn source(case: usize) -> String {
    let call = CALLS[case - 1];
    format!(
        "import {{css, keyframes}} from '@devup-ui/core'
const yellow = css({{ color: 'yellow', _hover: {{ color: 'red' }} }})
const azure = css({{ color: 'azure', bg: 'black' }})
const wide = css({{ color: ['a1', 'b1'] }})
const ordered = css({{ color: 'navy', styleOrder: 3 }})
const heading = css({{ typography: 'heading' }})
const body = css({{ typography: 'body' }})
const chained = css(yellow, azure)
const fade = keyframes({{ from: {{ opacity: 0 }} }})
export const result = {call}
"
    )
}

pub const IMPORTED_BASE: &str = "import { css } from '@devup-ui/react';
const brand = 'teal';
export const base = css({ color: brand, _hover: { color: 'red' } });
export const danger = css({ color: 'crimson' });";

pub const IMPORTED_APP: &str = "import { css } from '@devup-ui/react';
import { base, danger } from './styles.ts';
export const a = css(base, danger);
export const b = css(danger, base, { m: 1 });";

pub const INLINE: &str = "import { css } from '@emotion/react';\nconst base = css({ color: 'green', margin: 4 });\nexport const a = (cond) => css(cond && css({ color: 'red' }), base);";
pub const TEXT: &str = "import { css } from '@emotion/react';\nconst mixin = css({ color: 'red', margin: 1 });\nexport const a = css`color: green; ${mixin}; padding: 2px;`;";
