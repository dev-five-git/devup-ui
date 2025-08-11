<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>


<h3 align="center">
    Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor
</h3>

---

<div>
<img src='https://img.shields.io/npm/v/@devup-ui/react'>
<img src='https://img.shields.io/bundlephobia/minzip/@devup-ui/react'>
<img alt="Github Checks" src="https://badgen.net/github/checks/dev-five-git/devup-ui"/>
<img alt="Apache-2.0 License" src="https://img.shields.io/github/license/dev-five-git/devup-ui"/>
<a href="https://www.npmjs.com/package/@devup-ui/react">
<img alt="NPM Downloads" src="https://img.shields.io/npm/dm/@devup-ui/react.svg?style=flat"/>
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

English | [한국어](README_ko.md)

## Install

```sh
npm install @devup-ui/react

# on next.js
npm install @devup-ui/next-plugin

# on vite
npm install @devup-ui/vite-plugin
```

## Features

- Preprocessor
- Zero Config
- Zero FOUC
- Zero Runtime
- RSC Support
- Must not use JavaScript, client-side logic, or hybrid solutions
- Support Library mode
- Zero Cost Dynamic Theme Support based on CSS Variables
- Theme with Typing
- Smallest size, fastest speed

## Inspirations

- Styled System
- Chakra UI
- Theme UI
- Vanilla Extract
- Rainbow Sprinkles
- Kuma UI

## Comparison Benchmarks

Next.js Build Time and Build Size (github action - ubuntu-latest)

| Library      | Version  | Build Time | Build Size      |
|--------------|----------|------------|-----------------|
| kuma-ui      | 1.5.9    | 20.606s    | 63,248,791b     |
| chakra-ui    | 3.24.2   | 29.358s    | 195,258,486b    |
| mui          | 7.3.1    | 21.631s    | 88,332,632b     |
| devup-ui     | 1.0.15   | 16.873s    | 53,729,988b     |

## How it works

Devup UI is a CSS in JS preprocessor that does not require runtime.
Devup UI eliminates the performance degradation of the browser through the CSS in JS preprocessor.
We develop a preprocessor that considers all grammatical cases.

```typescript
const before = <Box bg={"red"}/>

const after = <div className="d0"/>
```

Variables are fully supported.

```typescript
const before = <Box bg={colorVariable}/>

const after = <div className="d0" style={{
    "--d0": colorVariable
}}/>
```

Various expressions and responsiveness are also fully supported.

```typescript
const before = <Box bg={["red", "blue", a > b ? "yellow" : variable]}/>

const after = <div className={`d0 d1 ${a > b ? "d2" : "d3"}`} style={{
    "--d2": variable
}}/>
```

Support Theme with Typing

`devup.json`

```json
{
  "theme": {
    "colors": {
      "default": {
        "text": "#000"
      },
      "dark": {
        "text": "white"
      }
    }
  }
}
```

```typescript
// Type Safe
<Text color="$text"/>
```

Support Responsive And Pseudo Selector

You can use responsive and pseudo selector.

```typescript
// Responsive with Selector
const box = <Box _hover={{bg: ["red", "blue"]}}/>

// Same
const box = <Box _hover={[{bg: "red"}, {bg: "blue"}]}/>
```
