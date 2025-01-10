# WIP

Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor

# Devup UI

English | [한국어](README_ko.md)

## Install

> npm install @devup-ui/react

## Features

- Preprocessor
- Zero Config
- Zero FOUC
- Zero Runtime
- RSC Support
- Must not use JavaScript, client-side logic, or hybrid solutions

## Inspirations

- Styled System
- Chakra UI
- Theme UI
- Vanilla Extract
- Rainbow Sprinkles
- Kuma UI

## Goal

Devup UI is a CSS in JS preprocessor that does not require runtime.
Devup UI eliminates the performance degradation of the browser through the CSS in JS preprocessor.
We develop a preprocessor that considers all grammatical cases.

```jsx
// Before
<Box bg={"red"}/>
// After
<Box className={"bg-red-0"}/>
```

Variables are fully supported.

```jsx
// Before
<Box bg={colorVariable}/>
// After
<Box className={"bg-0"} style={{
    "--bg-0": colorVariable
}}/>
```

Various expressions and responsiveness are also fully supported.

```jsx
// Before
<Box bg={["red", "blue", a > b ? "yellow" : variable]}/>
// After
<Box className={`bg-red-0 bg-blue-1 ${a > b ? "bg-yellow-2" : "bg-2"}`} style={{
    "--bg-2": variable
}}/>
```
