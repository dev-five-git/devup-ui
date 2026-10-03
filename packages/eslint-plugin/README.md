# @devup-ui/eslint-plugin

ESLint rules that check code the way the Devup UI build reads it. A rule reports and fixes only what the build reads as styles, and reports what the build rejects before the build does.

## Usage

```js
// eslint.config.mjs
import devupUi from '@devup-ui/eslint-plugin'

export default [...devupUi.configs.recommended]
```

## Rules

| Rule                                                                           | Checks                                                                                                                                                                         | Fix |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --- |
| [`css-utils-literal-only`](src/rules/css-utils-literal-only/README.md)         | values of `css`, `globalCss`, `keyframes`, `createGlobalStyle` and `<Global styles>` are known at build time, and `.css.ts` stylesheets read no global that does not exist there | no  |
| [`no-runtime-read`](src/rules/no-runtime-read/README.md)                       | what the build compiles away (`css`, `keyframes`, `styled`, `Box`...) is read only where it is called or rendered                                                              | no  |
| [`no-duplicate-value`](src/rules/no-duplicate-value/README.md)                 | consecutive duplicate values of a responsive array                                                                                                                             | yes |
| [`no-useless-responsive`](src/rules/no-useless-responsive/README.md)           | responsive arrays with a single value                                                                                                                                          | yes |
| [`no-useless-tailing-nulls`](src/rules/no-useless-tailing-nulls/README.md)     | trailing `null`s of a responsive array                                                                                                                                         | yes |
| [`no-typography-token-prefix`](src/rules/no-typography-token-prefix/README.md) | `$` before a `typography` key                                                                                                                                                  | yes |
| [`prefer-media-shorthand`](src/rules/prefer-media-shorthand/README.md)         | `_media` entries and `@media` keys that have a shorthand prop                                                                                                                  | yes |
| [`style-order-range`](src/rules/style-order-range/README.md)                   | `styleOrder` is a number from 1 to 254                                                                                                                                         | no  |

## What counts as a style

The rules follow the build: the style props of Devup UI components, the arguments of `css`, `globalCss`, `keyframes` and `createGlobalStyle`, and the same through the import aliases the build compiles by default: `@emotion/react`, `@emotion/styled`, `styled-components` and `@vanilla-extract/css` (`styled`, `Global`, `style`, `globalStyle`). Props the component passes through (`data-*`, `aria-*`, handlers, `props`, `styleVars`), `.attrs()` arguments and plain data elsewhere are left alone. Vanilla-extract stylesheets (`.css.ts`, `.css.js`) are run as they are by the build, so the style rules skip them.
