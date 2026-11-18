# Benchmark fixtures

Each comparison fixture renders the same page, so build time and CSS size are
measured for the same styling. The devup-ui-only stress apps
(`next-devup-ui-collapse`, `next-devup-ui-large*`, and the extra
`vinext-devup-ui` routes) are not part of this comparison.

## Page spec

| Element                                   | Styling                                                                                                                                                        |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<p>` Track & field champions:            | Inline `style={{ backgroundColor: 'blue' }}`, the same in every fixture                                                                                        |
| `<section data-testid="box">`             | `background: var(--text)`, `color` from state (`yellow`, then `blue`), `cursor: pointer`, `font-size: 32px`, `position: relative`, 28px top and bottom padding |
| `<section>:hover`                         | `background: yellow`, `cursor: cell`                                                                                                                           |
| Two `<div>hello</div>` inside the section | Unstyled                                                                                                                                                       |
| `<span>text</span>`                       | `color: var(--text)`                                                                                                                                           |
| `<div>hello</div>`                        | `color: green` when enabled, otherwise `blue`, `font-size: 32px`, `padding-right: 20px`                                                                        |
| `<div>hello</div>`                        | `font-size: 12px`, and `32px` at `min-width: 480px`, the second Devup UI breakpoint                                                                            |
| `<button>Change</button>`                 | Unstyled; switches the color to `blue` and toggles the enabled state                                                                                           |

No fixture applies a CSS reset. `--text` is left undefined, as in the Devup UI
fixture, which has no `devup.json`.

## How each library expresses it

| Fixture                                                                        | Notes                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `next-devup-ui`, `-single`, `-turbo`, `-single-turbo`, `vinext-devup-ui` (`/`) | Style props. A numeric `fontSize` is scaled by 4, so the page uses `fontSize={8}` for 32px and `fontSize={[3, 8]}` for 12px/32px. The dynamic color compiles to a CSS variable.                                                                                                                                                                 |
| `next-vanilla-extract`, `next-vanilla-extract-devup-ui`                        | `style()` with `@media`. The dynamic color goes through a `--box-color` variable that the page sets inline, since `@vanilla-extract/dynamic` is not a dependency.                                                                                                                                                                               |
| `next-tailwind`, `next-tailwind-turbo`                                         | Arbitrary values (`text-[32px]`, `bg-(--text)`, `min-[480px]:`) for the exact values, and `text-(--box-color)` with an inline variable for the dynamic color. Preflight is left out by importing only `theme.css` and `utilities.css`. Tailwind wraps `hover:` in `@media (hover: hover)`.                                                      |
| `next-tailwind-turbo-devup-ui`                                                 | The same classes compiled by Devup UI, except `sm:` for the 480px breakpoint, because `sm` maps to the second Devup UI breakpoint and `min-[…]` is not supported there. Hover is also wrapped in `@media (hover: hover)`.                                                                                                                       |
| `next-stylex`, `next-stylex-turbo`, `next-stylex-turbo-devup-ui`               | `stylex.create` with a dynamic style function for the color and a media query on `fontSize`. StyleX adds `:not(#\#)` specificity guards and an `@property` rule for its dynamic variable.                                                                                                                                                       |
| `next-kuma-ui`                                                                 | Style props, with `kuma.config.ts` setting the `sm` breakpoint to 480px. Kuma UI falls back to its runtime for the dynamic and conditional colors, so those rules are not in the static CSS.                                                                                                                                                    |
| `next-panda-css`                                                               | Style props with string sizes, `preflight: false`, and `sm` set to `30rem` (480px). Panda cannot extract a runtime value, so the color uses a `--box-color` variable. Its hover selector also matches `[data-hover]`, and the CSS still contains the `@pandacss/preset-panda` token variables.                                                  |
| `next-chakra-ui`                                                               | Style props with string sizes and the default `sm` breakpoint (30rem, 480px). Preflight is turned off with `createSystem(defaultConfig, { preflight: false })`, but Chakra still injects its token variables and global styles at runtime, and wraps hover in `@media (hover: hover)`. Styles are injected by Emotion, so there is no CSS file. |
| `next-mui`                                                                     | `sx` with `bgcolor` and a raw `@media (min-width: 480px)` key, since the default MUI `sm` breakpoint is 600px. Styles are injected by Emotion, so there is no CSS file.                                                                                                                                                                         |

All Next.js fixtures use `next.config.ts`, except `next-devup-ui-single-turbo`
and the mixed `*-devup-ui` fixtures, which keep `next.config.mjs`. A
`next.config.ts` is part of the TypeScript program, so the extension changes
the size of `.tsbuildinfo` (about 28 KB).
