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

[English](README.md) | 한국어

## 설치

```sh
npm install @devup-ui/react

# on next.js
npm install @devup-ui/next-plugin

# on vite
npm install @devup-ui/vite-plugin
```

## 기능

- 전처리기 - Devup UI 는 모든 코드를 전처리하여 성능 저하의 원인을 원천적으로 제거합니다.
- Zero Config - Devup UI 는 설정 파일이 필요 없습니다.
- Zero FOUC - Devup UI 는 FOUC를 완전히 제거합니다. 또한 방지를 위한 Provider 등 추가 설정이 필요 없습니다.
- Zero Runtime - Devup UI 는 런타임이 필요 없습니다.
- RSC Support - Devup UI 는 RSC를 지원합니다.
- Must not use JavaScript, client-side logic, or hybrid solutions - Devup UI 는 JavaScript, 클라이언트 사이드 로직, 혼합 솔루션을 사용하지
  않습니다.
- 라이브러리 모드 지원
- 타이핑 지원되는 테마
- 가장 작은 크기, 가장 빠른 속도

## 영감

- Styled System - 문법적인 부분에서 영감을 받았습니다.
- Chakra UI - 문법적인 부분에서 영감을 받았습니다.
- Theme UI - 전체적인 시스템적인 부분에서 영감을 받았습니다.
- Vanilla Extract - 전처리기 부분에서 영감을 받았습니다.
- Rainbow Sprinkles - 전체적인 시스템적인 부분에서 영감을 받았습니다.
- Kuma UI - 문법적인 부분과 방법론에서 영감을 받았습니다.

## 비교 벤치마크

Next.js Build Time and Build Size (github action - ubuntu-latest)

| 라이브러리  |  버전    | 빌드 시간   | 빌드 사이즈        |
|------------|---------|------------|------------------|
| kuma-ui      | 1.5.9    | 20.606s    | 63,248,791b     |
| chakra-ui    | 3.24.2   | 29.358s    | 195,258,486b    |
| mui          | 7.3.1    | 21.631s    | 88,332,632b     |
| devup-ui     | 1.0.15   | 16.873s    | 53,729,988b     |

## 작동 원리

Devup UI는 런타임이 필요 없는 CSS in JS 전처리기입니다.
Devup UI는 CSS in JS 전처리기를 통하여 브라우저의 성능 저하를 원천적으로 제거합니다.
모든 문법적 경우의 수를 고려하여 전처리기를 개발합니다.

```typescript
const before = <Box bg={"red"}/>

const after = <div className="d0"/>
```

변수 사용도 완전히 지원합니다.

```typescript
const before = <Box bg={colorVariable}/>

const after = <div className="d0" style={{
    "--d0": colorVariable
}}/>
```

다양한 표현식과 반응형도 모두 지원합니다.

```typescript
const before = <Box bg={["red", "blue", a > b ? "yellow" : variable]}/>

const after = <div className={`d0 d1 ${a > b ? "d2" : "d3"}`} style={{
    "--d2": variable
}}/>
```

타이핑이 되는 테마를 지원합니다.

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

반응형과 가상 선택자도 지원합니다.

물론 동시 사용도 가능합니다.

```typescript
// Responsive with Selector
const box = <Box _hover={{bg: ["red", "blue"]}}/>

// Same
const box = <Box _hover={[{bg: "red"}, {bg: "blue"}]}/>
```

## 기여 방법

### 요구 사항
- Rust 컴파일러 설치
- pnpm 패키지 매니저 설치

### 개발용 설치
개발 환경을 위해 아래 패키지들을 설치합니다.
```sh
pnpm i
pnpm build
cargo install cargo-tarpaulin
cargo install wasm-pack
```
설치 후 pnpm test를 실행하여 문제가 없는지 확인합니다.