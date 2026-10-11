# style-order-range

Checks build-time `styleOrder` metadata on the APIs that consume it.

## Rule Details

An explicit value must be an integer ECMAScript Number from 1 through 254,
or a canonical decimal string in that range. Strings must not contain signs,
spaces, leading zeroes, units or decimal/exponent notation. Cooked template
strings follow the same rule. Static arithmetic, coercions and constant
bindings are checked by value: `+'01'` and `+true` are Number `1`, whereas
`'01'` and `true` are invalid explicit values.

Class styles (`css`, `styled`, JSX style props and Emotion's `css` prop) may
choose valid orders with a runtime ternary or `&&`; the absent branch of `&&`
is implicit, not an explicitly written `null`, `false` or `undefined`.
Runtime `||` and `??` are unsupported. A static expression using those operators
is checked by its folded value.

Globals (`globalCss`, `globalStyle`, `createGlobalStyle`, Emotion `Global`)
accept static order metadata at the root and inside selector/at-rule objects,
but cannot select an order at runtime. Keyframes never accept order metadata,
including frame and nested objects. `globalCss.fontFaces` descriptors likewise
reject order metadata because it has no effect. Native StyleX declaration bodies also
reject it; namespace keys and `defineVars` variable names are not declarations.

The rule resolves lexical imports and their aliases from Devup UI, its compat
entry, Emotion, styled-components, vanilla-extract and StyleX. It ignores
shadowed imports, unrelated functions and pass-through data such as `props`
and `vars`. It does not execute user code or load another module's values;
immutable imported values and structurally pure local function results are left
to the build when their exact range cannot be proved locally. Calls using runtime
parameters, mutable locals, imported functions or effectful bodies are still
reported. Emotion `ClassNames`' direct child callback is recognized only for its
destructured `css` binding (including a local alias). Statically known argument
spread arrays are expanded; selector/layer record keys named `styleOrder` remain
record keys rather than metadata. The build remains authoritative for deferred
range proofs.

Accepted CSS-text routes check real `style-order` declarations (and the
`styleOrder` alias), including tagged templates, string arguments and nested
selector/at-rule text. A lexical scan preserves comments, quotes, escapes,
functions, attribute selectors and custom-property brace streams; content
strings, ordinary property values, data records and selectors named
`styleOrder` are not directives. Diagnostics refer to the original declaration
token or interpolation, not normalized text.

Bare CSS numeric tokens follow Number semantics (`+2.0` is valid); quoted CSS
strings are decoded once and must be canonical (`"02"` is invalid). Exact
interpolations preserve primitive types. Mixed finite interpolations are
strings, so their resulting values must be canonical. Only styled APIs may
inspect function interpolations as callbacks; no callback is executed by the
rule. Global text requires static metadata, and frame/font-face scopes reject
metadata regardless of its value. Vanilla-extract `style` strings in `.css.ts`
modules remain class composition rather than CSS text; accepted alias text
routes outside those modules and `globalStyle` text are checked.

### Examples of **incorrect** code for this rule:

```jsx
// Zero and negative values
<Box styleOrder={0} />
<Box styleOrder={-1} />
<Box styleOrder="-5" />

// Values greater than or equal to 255
<Box styleOrder={255} />
<Box styleOrder={256} />
<Box styleOrder="300" />

// Non-numeric values
<Box styleOrder="abc" />
<Box styleOrder={undefined} />
<Box styleOrder="01" />
<Box styleOrder={1.5} />
globalCss({ body: { styleOrder: active ? 1 : 2 } })
keyframes({ from: { styleOrder: 1, opacity: 0 } })
```

### Examples of **correct** code for this rule:

```jsx
// Valid range values (0 < value < 255)
<Box styleOrder={1} />
<Box styleOrder={254} />
<Box styleOrder={128} />
<Box styleOrder="100" />
<Box styleOrder="1" />
<Box styleOrder="254" />
css({ styleOrder: active ? 1 : 2 })
styled.div({ styleOrder: active && 2 })
globalCss({ styleOrder: 1, body: { styleOrder: 2 } })
```

## Where it applies

The rule checks metadata at recognized style API sites, not every property named
`styleOrder` in the file. This includes direct component style props, consumed
style arguments and nested selector/at-rule style objects. Globals require static
orders there; keyframes, font-face descriptors and native StyleX declarations
reject metadata even in nested style scopes. Selector/layer record keys named
`styleOrder` are not metadata, but metadata inside their style values is checked.
Ordinary `data-*` props, pass-through `props` and `vars`, unrelated data objects
and ordinary elements passed as props are ignored. A recognized styling call or
component inside a prop expression is still checked independently at its own
style site.

## When Not To Use It

If you don't use `styleOrder` props or want to allow any value range, you can disable this rule.
