<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/components'>
<img src='https://img.shields.io/bundlephobia/minzip/@devup-ui/components'>
<img alt="Github Checks" src="https://badgen.net/github/checks/dev-five-git/devup-ui"/>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/components">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/components.svg?style=flat"/>
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

# @devup-ui/components

Ready-made React components built with [Devup UI](https://devup-ui.com):
`Button`, `Checkbox`, `Input`, `Radio`, `RadioGroup`, `Select`, `Stepper`,
`Textarea` and `Toggle`. Their styles are compiled by the Devup UI build plugin
like your own code, so they ship no styling runtime.

## Install

```sh
npm install @devup-ui/components @devup-ui/react

# and the build plugin for your bundler, for example
npm install @devup-ui/vite-plugin
```

The components need the Devup UI build plugin
([Vite](https://www.npmjs.com/package/@devup-ui/vite-plugin),
[Next.js](https://www.npmjs.com/package/@devup-ui/next-plugin),
[Rsbuild](https://www.npmjs.com/package/@devup-ui/rsbuild-plugin),
[Webpack](https://www.npmjs.com/package/@devup-ui/webpack-plugin) or
[Bun](https://www.npmjs.com/package/@devup-ui/bun-plugin)). The plugins compile
`@devup-ui/*` packages in `node_modules` by default, so no extra `include`
option is needed. React 18 and 19 are supported.

## Usage

```tsx
import { Button, Input, Toggle } from '@devup-ui/components'

export function Form() {
  return (
    <form>
      <Input aria-label="Email" name="email" placeholder="Email" />
      <Toggle aria-label="Subscribe" name="subscribe" />
      <Button type="submit" variant="primary">
        Submit
      </Button>
    </form>
  )
}
```

## Exports

| Export                                                                                                        | What it is                                    |
| ------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `Button`                                                                                                      | Button with variants, sizes and loading state |
| `Checkbox`                                                                                                    | Checkbox with a label                         |
| `Input`, `Textarea`                                                                                           | Text fields with error messages               |
| `Radio`, `RadioGroup`                                                                                         | Radio buttons and a group sharing one value   |
| `Select`, `SelectTrigger`, `SelectContainer`, `SelectOption`, `SelectDivider`, `useSelect`, `SelectContext`   | Single and multiple selection popup           |
| `Stepper`, `StepperContainer`, `StepperDecreaseButton`, `StepperIncreaseButton`, `StepperInput`, `useStepper` | Numeric stepper                               |
| `Toggle`                                                                                                      | Switch storing a boolean, submitted in forms  |

Most components accept `className`, `style`, a `colors` object that overrides
their colors through CSS variables, and the props of the element they render.

See the [component documentation](https://devup-ui.com/components/overview)
for each component's props and live examples.

## License

Apache-2.0
