# no-unknown-style-spread

Opt-in, type-aware guard for JSX spreads on Devup primitives. The build still
decides what it can extract; this rule does not change runtime pass-through,
`className`, or `style` merging.

The rule reports only when **both** are true:

1. The expression is provably runtime-rooted: a function/arrow/method parameter
   (including destructured/rest parameters), catch parameter, `arguments`, `this`,
   or a never-reassigned local alias of one. Members preserve that origin.
   An object with a runtime-rooted spread also qualifies. Runtime-first `||` and
   `??` remain runtime-rooted even with a known fallback: the build passes the
   spread through instead of extracting that fallback's styles. `&&` and
   conditional expressions require **every value branch** to qualify.
2. Its TypeScript type can carry Devup styles. The schema comes from the real
   primitive's public props, including selector/theme/custom shorthand types.
   Named, union, intersection, constrained generic and index types are inspected;
   `any`, `unknown`, `{}`, `object`, and unconstrained generics can carry styles.

```tsx
import { Box, type DevupProps } from '@devup-ui/react'

function unsafe(styles: DevupProps) {
  return <Box {...styles} /> // report: runtime origin and style-capable type
}

function explicit(color: string) {
  return <Box {...{ bg: color }} /> // allowed: known keys, dynamic value
}

function dom(props: React.ImgHTMLAttributes<HTMLImageElement>) {
  return <Box as="img" {...props} /> // allowed, including DOM color/width
}
```

React DOM-origin properties do not count as Devup styles, including overlapping
`color`/`width` attributes and mapped DOM types (`Pick`, `Omit`, `Partial`, ...).
An authored `{ color?: string }` property is not proven DOM-origin and does count.
Only actual intrinsic attribute declarations are exempt: an event-value property
such as `React.PointerEvent.width` is not a DOM attribute just because it is
declared by React. Intersections use their effective properties, so an
undefined-only style property does not emit a diagnostic.
Generic intersections retain open constraints: `HTMLAttributes & T` reports when
`T` is unconstrained or constrained to `object`, `{}`, or `unknown`. Concrete
`HTMLAttributes & object` and `HTMLAttributes & {}` remain DOM-only, as do generics
constrained to concrete DOM/nonstyle shapes. Finite constrained keys eliminated
by an effective undefined-only intersection are not revived.
Adding a style-bearing property to a DOM type via intersection or augmentation
does not make that property exempt. `className`, `style`, event handlers,
data/ARIA attributes and other pass-through props do not count as style keys.

An object containing a runtime spread is classified using the object's resulting
type: `{ ...runtimeDomProps, p: 2 }` qualifies if that type includes `p`. In
contrast, `{ bg: runtimeColor }` alone is not a runtime-rooted spread.

The real build/lint agreement tests cover both shared and per-file CSS modes:

- Local destructuring defaults, parameter destructuring defaults, and plain
  parameter defaults still pass runtime styles through, even when the default
  is an imported constant. Style-capable spreads of these bindings report.
- `rest ?? knownImport` and `rest || knownImport` pass through without extracting
  the fallback's styles, so style-capable runtime-first spreads report. Literal
  and computed fallbacks have the same verified runtime-first behavior.
- `knownImport ?? rest` and `knownImport || rest` compose the known object and do
  not report. A known imported object by itself also composes normally.
- The two mixed conditional directions (`condition ? knownImport : rest` and
  `condition ? rest : knownImport`) already produce located build errors. This
  rule adds no extra diagnostic for those mixed conditional cases.

Literals, imports, standalone calls (which may be build-evaluable), const/import
members, reassigned bindings, and otherwise unclassified mixed expressions are
left to the build. No evaluator is duplicated; the logical change does not infer
whether a call or import by itself is runtime-only.

The target must be an imported Devup primitive (named/renamed or namespace/default
package access); shadowed names and unrelated components are excluded. The rule
does not infer styled-created components or variable component aliases.
Invalid/unresolved primitive declarations produce a type-resolution error,
not an empty-schema pass. There is no autofix: rewriting spreads can change
evaluation order.

For a runtime spread intended to forward attributes, constrain its type to the
concrete DOM/nonstyle shape it actually carries, or pass style props explicitly.
Partially overlapping string-template index domains still count as style-capable;
disjoint data/ARIA, numeric and symbol key domains do not.

Effective nullish/never-only properties and indexes exclude only the style keys
they eliminate. Finite keys use public checker queries; infinite string-template
domains use exact string-pattern intersection/subtraction, matching UTF-16 code
units as TypeScript does. Style keys are not all finite: at-rule and theme index
domains also come from the primitive's public schema.

For unsupported template domains or masks (including numeric placeholders), a
bounded fallback fills the represented domain texts with a few concrete seeds
(`''`, `screen`, `(display: grid)`, `0`, `1`). Each candidate must be verified by
the actual checker as belonging to **both** positive domains and **none** of the
exclusions before it proves style capability. For example, a numeric mask cannot
hide the surviving `@media screen` key. Whole-domain masks still suppress reports.
This is not numeric-template grammar or algebra: if neither the supported matcher
nor a verified witness proves carriage, only that narrow query is left
unreported. Other finite keys and index domains are still checked normally.

## Configuration

```js
import devupUi from '@devup-ui/eslint-plugin'

export default [
  ...devupUi.configs.recommendedTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
]
```

The preset includes `recommended` plus this rule for TS/TSX/MTS/CTS files. It
registers a stable separate namespace:
`@devup-ui/type-checked/no-unknown-style-spread`. This lets it compose with the
unchanged syntax-only preset without redefining `@devup-ui`. An existing typed
`parserOptions.project` setup is also supported. Enabling the rule without a
program throws the standard typescript-eslint configuration error, even in a
file without spreads. The named rule export is `rules.noUnknownStyleSpread`.
