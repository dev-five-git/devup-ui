<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    CSS-in-JS의 미래 — 제로 런타임, 완전한 문법 지원
</h3>

<p align="center">
    <strong>Zero Config · Zero FOUC · Zero Runtime · 모든 CSS-in-JS 문법 완벽 지원</strong>
</p>

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

## 왜 Devup UI인가?

**Devup UI는 단순한 CSS-in-JS 라이브러리가 아닙니다 — CSS-in-JS의 미래 그 자체입니다.**

기존 CSS-in-JS 솔루션들은 개발자 경험과 성능 사이에서 타협을 강요했습니다. Devup UI는 Rust 기반 전처리기를 통해 모든 스타일을 빌드 타임에 처리함으로써 이 트레이드오프를 완전히 제거합니다.

- **완전한 문법 지원**: 변수, 조건문, 반응형 배열, 가상 선택자 등 모든 CSS-in-JS 패턴을 완벽하게 지원
- **익숙한 API**: styled-components, Emotion과 호환되는 `styled()` API 제공
- **진정한 제로 런타임**: 런타임에서 스타일링을 위한 JavaScript 실행이 전혀 없습니다
- **가장 작은 번들 크기**: 최적화된 클래스명(`a`, `b`, ... `aa`, `ab`)으로 CSS 출력 최소화
- **가장 빠른 빌드 속도**: Rust + WebAssembly가 제공하는 압도적인 전처리 성능

## 설치

```sh
npm install @devup-ui/react

# Next.js
npm install @devup-ui/next-plugin

# Vite
npm install @devup-ui/vite-plugin

# Rsbuild
npm install @devup-ui/rsbuild-plugin

# Webpack
npm install @devup-ui/webpack-plugin
```

## 기능

- **전처리기** — 모든 CSS 추출이 빌드 타임에 완료됩니다
- **Zero Config** — 별도 설정 없이 바로 사용 가능
- **Zero FOUC** — 스타일 깜빡임 없음, Provider 불필요
- **Zero Runtime** — 스타일링을 위한 클라이언트 JavaScript 실행 없음
- **RSC 지원** — React Server Components 완벽 호환
- **라이브러리 모드** — 추출된 스타일과 함께 컴포넌트 라이브러리 빌드 가능
- **동적 테마** — CSS 변수 기반 제로 코스트 테마 전환
- **타입 세이프 테마** — 테마 토큰에 대한 완전한 TypeScript 지원
- **가장 작고 빠름** — 벤치마크로 증명됨

## 비교 벤치마크

Next.js Build Time and Build Size (github action - ubuntu-latest)

| 라이브러리                         | 버전   | 빌드 시간  | 빌드 사이즈          |
| ---------------------------------- | ------ | ---------- | -------------------- |
| tailwindcss                        | 4.1.13 | 19.31s     | 59,521,539 bytes     |
| styleX                             | 0.15.4 | 41.78s     | 86,869,452 bytes     |
| vanilla-extract                    | 1.17.4 | 19.50s     | 61,494,033 bytes     |
| kuma-ui                            | 1.5.9  | 20.93s     | 69,924,179 bytes     |
| panda-css                          | 1.3.1  | 20.64s     | 64,573,260 bytes     |
| chakra-ui                          | 3.27.0 | 28.81s     | 222,435,802 bytes    |
| mui                                | 7.3.2  | 20.86s     | 97,964,458 bytes     |
| **devup-ui(per-file css)**         | 1.0.18 | **16.90s** | 59,540,459 bytes     |
| **devup-ui(single css)**           | 1.0.18 | **17.05s** | **59,520,196 bytes** |
| tailwindcss(turbopack)             | 4.1.13 | 6.72s      | 5,355,082 bytes      |
| **devup-ui(single css+turbopack)** | 1.0.18 | 10.34s     | **4,772,050 bytes**  |

## 작동 원리

Devup UI는 빌드 타임에 컴포넌트를 변환합니다. 클래스명은 CSS 크기를 최소화하기 위해 압축된 base-37 인코딩을 사용하여 생성됩니다.

**기본 변환:**

```tsx
// 개발자가 작성:
const example = <Box _hover={{ bg: 'blue' }} bg="red" p={4} />

// Devup UI가 생성:
const generated = <div className="a b c" />

// CSS:
// .a { background-color: red; }
// .b { padding: 1rem; }
// .c:hover { background-color: blue; }
```

**동적 값은 CSS 변수로 변환:**

```tsx
// 개발자가 작성:
const example = <Box bg={colorVariable} />

// Devup UI가 생성:
const generated = <div className="a" style={{ '--a': colorVariable }} />

// CSS:
// .a { background-color: var(--a); }
```

**복잡한 표현식과 반응형 배열 — 완벽 지원:**

```tsx
// 개발자가 작성:
const example = <Box bg={['red', 'blue', isActive ? 'green' : dynamicColor]} />

// Devup UI가 생성:
const generated = (
  <div
    className={`a b ${isActive ? 'c' : 'd'}`}
    style={{ '--d': dynamicColor }}
  />
)

// 각 브레이크포인트에 대한 반응형 CSS 생성
```

**타입 세이프 테마:**

`devup.json`

```json
{
  "theme": {
    "colors": {
      "default": {
        "primary": "#0070f3",
        "text": "#000"
      },
      "dark": {
        "primary": "#3291ff",
        "text": "#fff"
      }
    },
    "typography": {
      "heading": {
        "fontFamily": "Pretendard",
        "fontSize": "24px",
        "fontWeight": 700,
        "lineHeight": 1.3
      }
    }
  }
}
```

```tsx
// 타입 세이프 테마 토큰
const textExample = <Text color="$primary" />
const boxExample = <Box typography="$heading" />
```

**반응형 + 가상 선택자 동시 사용:**

```tsx
// 반응형과 가상 선택자 함께 사용
const example = <Box _hover={{ bg: ['red', 'blue'] }} />

// 동일한 표현
const example2 = <Box _hover={[{ bg: 'red' }, { bg: 'blue' }]} />
```

**styled-components / Emotion 호환 `styled()` API:**

```tsx
import { styled } from '@devup-ui/react'

// styled-components, Emotion 사용자에게 익숙한 문법
const Card = styled('div', {
  bg: 'white',
  p: 4, // 4 * 4 = 16px
  borderRadius: '8px',
  boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)',
  _hover: {
    boxShadow: '0 10px 15px rgba(0, 0, 0, 0.1)',
  },
})

// 버튼 예시
const Button = styled('button', {
  px: 4, // 4 * 4 = 16px
  py: 2, // 2 * 4 = 8px
  borderRadius: '4px',
  cursor: 'pointer',
})

// 사용
const cardExample = <Card>Content</Card>
const buttonExample = <Button>Click me</Button>
```

## 영감

- Styled System — 문법적인 부분에서 영감을 받았습니다
- Chakra UI — 문법적인 부분에서 영감을 받았습니다
- Theme UI — 전체적인 시스템적인 부분에서 영감을 받았습니다
- Vanilla Extract — 전처리기 부분에서 영감을 받았습니다
- Rainbow Sprinkles — 전체적인 시스템적인 부분에서 영감을 받았습니다
- Kuma UI — 문법적인 부분과 방법론에서 영감을 받았습니다

## 기여 방법

### 요구 사항

- [Node.js](https://nodejs.org) (LTS 버전 권장)
- [Rust](https://rustup.rs) 컴파일러
- [Bun](https://bun.sh) 패키지 매니저

### 개발 환경 설정

개발 환경을 위해 아래 패키지들을 설치합니다:

```sh
bun install
bun run build
cargo install cargo-tarpaulin
cargo install wasm-pack
```

설치 후 `bun run test`를 실행하여 문제가 없는지 확인합니다.
