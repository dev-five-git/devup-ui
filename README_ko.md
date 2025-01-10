Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor

# Devup UI

[English](README.md) | 한국어

## Install

> npm install @devup-ui/react

## 기능

- 전처리기 - Devup UI 는 모든 코드를 전처리하여 성능 저하의 원인을 원천적으로 제거합니다.
- Zero Config - Devup UI 는 설정 파일이 필요 없습니다.
- Zero FOUC - Devup UI 는 FOUC를 완전히 제거합니다. 또한 방지를 위한 Provider 등 추가 설정이 필요 없습니다.
- Zero Runtime - Devup UI 는 런타임이 필요 없습니다.
- RSC Support - Devup UI 는 RSC를 지원합니다.
- Must not use JavaScript, client-side logic, or hybrid solutions - Devup UI 는 JavaScript, 클라이언트 사이드 로직, 혼합 솔루션을 사용하지
  않습니다.

## 영감

- Styled System - 문법적인 부분에서 영감을 받았습니다.
- Chakra UI - 문법적인 부분에서 영감을 받았습니다.
- Theme UI - 전체적인 시스템적인 부분에서 영감을 받았습니다.
- Vanilla Extract - 전처리기 부분에서 영감을 받았습니다.
- Rainbow Sprinkles - 전체적인 시스템적인 부분에서 영감을 받았습니다.
- Kuma UI - 문법적인 부분과 방법론에서 영감을 받았습니다.

## 목표

Devup UI는 런타임이 필요 없는 CSS in JS 전처리기입니다.
Devup UI는 CSS in JS 전처리기를 통하여 브라우저의 성능 저하를 원천적으로 제거합니다.
모든 문법적 경우의 수를 고려하여 전처리기를 개발합니다.

```jsx
// Before
<Box bg={"red"}/>
// After
<Box className={"bg-red-0"}/>
```

변수 사용도 완전히 지원합니다.

```jsx
// Before
<Box bg={colorVariable}/>
// After
<Box className={"bg-0"} style={{
    "--bg-0": colorVariable
}}/>
```

다양한 표현식과 반응형도 모두 지원합니다.

```jsx
// Before
<Box bg={["red", "blue", a > b ? "yellow" : variable]}/>
// After
<Box className={`bg-red-0 bg-blue-1 ${a > b ? "bg-yellow-2" : "bg-2"}`} style={{
    "--bg-2": variable
}}/>
```
