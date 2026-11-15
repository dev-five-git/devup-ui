<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/reset-css'>
<img src='https://img.shields.io/bundlephobia/minzip/@devup-ui/reset-css'>
<img alt="Github Checks" src="https://badgen.net/github/checks/dev-five-git/devup-ui"/>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/reset-css">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/reset-css.svg?style=flat"/>
</a>
<a href="https://badgen.net/github/stars/dev-five-git/devup-ui">
<img alt="Github Stars" src="https://badgen.net/github/stars/dev-five-git/devup-ui" />
</a>
<a href="https://discord.gg/8zjcGc7cWh">
<img alt="Discord" src="https://img.shields.io/discord/1321362173619994644.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
</a>
<a href="https://codecov.io/gh/dev-five-git/devup-ui" >
 <img src="https://codecov.io/gh/dev-five-git/devup-ui/graph/badge.svg?token=8I5GMB2X5B"/>
</a>
</div>

---

English | [한국어](https://github.com/dev-five-git/devup-ui/blob/main/README_ko.md)

# @devup-ui/reset-css

A CSS reset for [Devup UI](https://devup-ui.com) apps, based on
[sanitize.css](https://github.com/csstools/sanitize.css). The rules are written
with `globalCss()` and compiled into your stylesheet by the Devup UI build
plugin, so the reset ships no runtime code.

## Install

```sh
npm install @devup-ui/reset-css @devup-ui/react
```

A Devup UI build plugin must be configured
([Vite](https://www.npmjs.com/package/@devup-ui/vite-plugin),
[Next.js](https://www.npmjs.com/package/@devup-ui/next-plugin),
[Rsbuild](https://www.npmjs.com/package/@devup-ui/rsbuild-plugin),
[Webpack](https://www.npmjs.com/package/@devup-ui/webpack-plugin) or
[Bun](https://www.npmjs.com/package/@devup-ui/bun-plugin)). The plugins compile
`@devup-ui/*` packages in `node_modules` by default.

## Usage

Import the package once, near the root of your app (for example in the root
layout), and call `resetCss()`:

```tsx
import { resetCss } from '@devup-ui/reset-css'

resetCss()
```

Importing the module is what adds the rules; `resetCss()` itself does nothing
at runtime and returns `undefined`. Calling it keeps the import from being
removed as unused, so the reset stays in the build.

## What it resets

The rules apply to the whole document. Most selectors are wrapped in
`:where()`, so they have zero specificity and any style of your own overrides
them. Among others, the reset:

- sets `box-sizing: border-box` on every element and pseudo-element
- removes the `body` margin and sets a `1.5` root line height
- removes list markers inside `nav`
- aligns `img`, `video`, `svg` and similar media to the middle of the line
- removes the default margin of form controls and lets a `textarea` resize
  only vertically
- hides `dialog:not([open])`, and keeps `[aria-hidden="false"][hidden]`
  content available to assistive technology while visually hiding it
- sets the `progress` cursor on `[aria-busy="true"]`, `pointer` on
  `[aria-controls]` and `not-allowed` on disabled elements

See [`src/index.ts`](https://github.com/dev-five-git/devup-ui/blob/main/packages/reset-css/src/index.ts)
for the complete list of rules.

## License

Apache-2.0
