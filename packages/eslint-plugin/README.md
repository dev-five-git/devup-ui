# @devup-ui/eslint-plugin

ESLint checks for Devup UI. Syntax-only rules follow the build's style positions and refusals. The opt-in typed spread guard additionally warns when style-capable runtime props would be passed through rather than styled.

## Usage

```js
// eslint.config.mjs
import devupUi from '@devup-ui/eslint-plugin'

export default [...devupUi.configs.recommended]
```

For the opt-in [type-aware unknown-spread guard](src/rules/no-unknown-style-spread/README.md),
use `configs.recommendedTypeChecked` and configure your lint project's parser services:

```js
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

The typed rule uses the separate `@devup-ui/type-checked` namespace so the presets
compose without redefining the existing plugin. `recommended` remains syntax-only.

The guard follows verified build value flow: style-capable runtime defaults and
runtime-first `rest || fallback` / `rest ?? fallback` spreads report because they
pass through without extracting styles. Known-first imported controls compose and
remain allowed. Mixed conditionals already refused by the build receive no extra
typed-spread diagnostic; unclassified expressions remain the build's decision.

## Rules

| Rule                                                                           | Checks                                                                                                                                                                         | Fix |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --- |
| [`css-utils-literal-only`](src/rules/css-utils-literal-only/README.md)         | values of `css`, `globalCss`, `keyframes`, `createGlobalStyle` and `<Global styles>` are known at build time, and `.css.ts` stylesheets read no global that does not exist there | no  |
| [`no-runtime-read`](src/rules/no-runtime-read/README.md)                       | what the build compiles away (`css`, `keyframes`, `styled`, `Box`...) is read only where it is called or rendered                                                              | no  |
| [`no-unknown-style-spread`](src/rules/no-unknown-style-spread/README.md)       | opt-in typed guard for runtime-rooted spreads whose types can carry Devup styles; concrete DOM-only props remain allowed                                                        | no  |
| [`no-duplicate-value`](src/rules/no-duplicate-value/README.md)                 | consecutive duplicate values of a responsive array                                                                                                                             | yes |
| [`no-useless-responsive`](src/rules/no-useless-responsive/README.md)           | responsive arrays with a single value                                                                                                                                          | yes |
| [`no-useless-tailing-nulls`](src/rules/no-useless-tailing-nulls/README.md)     | trailing `null`s of a responsive array                                                                                                                                         | yes |
| [`no-typography-token-prefix`](src/rules/no-typography-token-prefix/README.md) | `$` before a `typography` key                                                                                                                                                  | yes |
| [`prefer-media-shorthand`](src/rules/prefer-media-shorthand/README.md)         | `_media` entries and `@media` keys that have a shorthand prop                                                                                                                  | yes |
| [`style-order-range`](src/rules/style-order-range/README.md)                   | `styleOrder` is a number from 1 to 254                                                                                                                                         | no  |

## What counts as a style

The rules follow the build: the style props of Devup UI components, the arguments of `css`, `globalCss`, `keyframes` and `createGlobalStyle`, and the same through the import aliases the build compiles by default: `@emotion/react`, `@emotion/styled`, `styled-components` and `@vanilla-extract/css` (`styled`, `Global`, `style`, `globalStyle`). Props the component passes through (`data-*`, `aria-*`, handlers, `props`, `styleVars`), `.attrs()` arguments and plain data elsewhere are left alone. Vanilla-extract stylesheets (`.css.ts`, `.css.js`) are run as they are by the build, so the style rules skip them.
