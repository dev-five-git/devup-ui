<div align="center">
  <img src="https://raw.githubusercontent.com/dev-five-git/devup-ui/main/media/logo.svg" alt="Devup UI logo" width="300" />
</div>

<h3 align="center">
    CSS-in-JS의 미래 — 제로 런타임, 강력한 기능
</h3>

<p align="center">
    <strong>Zero Config · Zero FOUC · Zero Runtime · 빌드 타임 CSS-in-JS</strong>
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

- **폭넓은 문법 지원**: 변수, 조건문, 반응형 배열, 가상 선택자, `styled()`, Emotion `css` prop 등을 빌드 타임에 컴파일합니다. 빌드가 알 수 없는 것은 조용히 빠지지 않고 [위치가 표시된 빌드 오류](https://devup-ui.com/docs/build-errors)가 됩니다 ([지원 문법과 한계](https://devup-ui.com/docs/limitations))
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

<!-- benchmark:start -->

[최신 CI 벤치마크](https://github.com/dev-five-git/devup-ui/actions/runs/44324447924) (커밋 `14819ae1`, Devup UI 1.0.44, 2026-11-18)는 `ubuntu-26.04`에서 Next.js 16.4.6로 실행했습니다. 모든 행은 순서를 돌려 가며 측정한 cold build 3회의 중앙값이며 샘플의 범위를 함께 보여줍니다. 모든 Next.js 빌드는 네이티브 TypeScript 7 CLI로 타입 검사를 수행합니다. 영어와 한국어 표는 같은 결과 파일 `benchmark-results.json`에서 생성합니다. CI는 main에 병합될 때마다 이 파일을 갱신하고 커밋마다 하나씩 `benchmark/history.json`에 기록하므로, 빌드가 느려지면 원인이 된 병합을 찾을 수 있습니다.

클라이언트 사이즈는 브라우저에 제공되는 JavaScript와 CSS의 압축 전 크기입니다: `.next/static/**/*.{js,css}` 또는 vinext `dist/client/**/*.{js,css}`. 빌드 사이즈(캐시 포함)는 빌드 출력 디렉터리(`.next`, vinext는 `dist`)의 모든 바이트입니다. 대부분이 번들러 캐시와 서버 출력이고 같은 빌드끼리도 수 KB씩 달라지므로, 사용자가 받는 크기가 아니라 이전 결과와의 연속성을 위해 남겨 둡니다. CSS 사이즈는 브라우저에 제공되는 CSS만 측정합니다: `.next/static/**/*.css`(Webpack `static/css`, Turbopack `static/chunks`) 또는 vinext `dist/client/**/*.css`. 캐시와 서버 CSS는 제외합니다. 브라우저 에셋 디렉터리가 없으면 오류이며, 존재하지만 CSS가 없으면 0입니다. 모든 사이즈는 샘플마다 측정해 중앙값으로 보여 주며, `benchmark-results.json`에는 샘플과 범위가 함께 남습니다. 클라이언트 사이즈를 기록하기 전에 측정한 행은 n/a로 표시합니다. 승인된 CSS 측정 보정은 명시적으로 지정한 `devup-ui`, `devup-ui-single`, `vinext-devup-ui` 행으로 제한됩니다. `bun benchmark-check.js --write-css-baseline devup-ui`는 기존 per-file 모드를 유지하며, `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui`는 쉼표로 구분한 인자 하나로 나머지 두 행만 선택합니다. 선택한 기존 행의 CSS 바이트만 바꾸고, `--write-baseline`과 달리 샘플 수, 빌드 바이트, 비율, 허용 오차, 선택하지 않은 모든 행과 보정 이력을 보존합니다. 이름 누락, 지원하지 않는 이름, 다른 쓰기 모드와의 혼용은 오류입니다.

Webpack:

| 라이브러리                       | 버전     | 빌드 시간 중앙값  | 범위              | 클라이언트 사이즈       | 빌드 사이즈(캐시 포함)     | CSS 사이즈      |
| --------------------------- | ------ | ---------- | --------------- | --------------- | ----------------- | ------------ |
| tailwindcss                 | 4.3.5  | 11.94s     | 11.86s - 12.02s | 924,183 bytes   | 69,758,622 bytes  | 717 bytes    |
| styleX                      | 0.19.2 | 24.36s     | 24.14s - 24.52s | 2,021,525 bytes | 97,727,611 bytes  | 661 bytes    |
| vanilla-extract             | 1.21.3 | 10.82s     | 10.79s - 10.85s | 923,721 bytes   | 70,448,102 bytes  | 398 bytes    |
| kuma-ui                     | 1.6.4  | 12.43s     | 12.42s - 12.46s | 956,158 bytes   | 77,760,538 bytes  | 354 bytes    |
| panda-css                   | 2.5.0  | 12.28s     | 12.23s - 12.32s | 980,282 bytes   | 75,835,577 bytes  | 18,567 bytes |
| chakra-ui                   | 3.38.0 | 18.52s     | 18.52s - 18.64s | 1,175,910 bytes | 211,192,005 bytes | 0 bytes      |
| mui                         | 9.5.0  | 13.14s     | 13.12s - 13.36s | 993,400 bytes   | 103,205,651 bytes | 0 bytes      |
| **devup-ui (per-file CSS)** | 1.0.44 | **10.50s** | 10.43s - 10.86s | 923,753 bytes   | 69,830,734 bytes  | 355 bytes    |
| **devup-ui (single CSS)**   | 1.0.44 | **10.55s** | 10.49s - 10.82s | 923,689 bytes   | 69,814,786 bytes  | 325 bytes    |

Turbopack:

| 라이브러리                                  | 버전     | 빌드 시간 중앙값 | 범위            | 클라이언트 사이즈     | 빌드 사이즈(캐시 포함)    | CSS 사이즈   |
| -------------------------------------- | ------ | --------- | ------------- | ------------- | ---------------- | --------- |
| tailwindcss                            | 4.3.5  | 4.42s     | 4.33s - 4.44s | 571,738 bytes | 30,530,536 bytes | 653 bytes |
| **devup-ui (direct APIs, single CSS)** | 1.0.44 | **4.26s** | 4.24s - 4.35s | 571,234 bytes | 28,917,131 bytes | 325 bytes |
| **devup-ui (static `.css.ts`)**        | 1.0.44 | **4.31s** | 4.29s - 4.38s | 571,331 bytes | 28,972,453 bytes | 376 bytes |

Turbopack 측정 범위는 서로 겹치므로 direct API 중앙값은 이 fixture에서 Tailwind보다 0.16s(3.7%) 빠르며 사실상 동률입니다. cold 샘플 3개는 Tailwind `4.44s, 4.42s, 4.33s`, direct Devup UI `4.35s, 4.24s, 4.26s`, static `.css.ts` `4.38s, 4.29s, 4.31s`입니다. fixture는 앱 구조만 비슷하고 스타일이 픽셀 단위로 같지는 않습니다. Tailwind는 첫 문단과 버튼에 더 많은 스타일을 주고, Devup UI는 타입이 있는 컴포넌트/스타일 props를 사용합니다. 규칙 단위 마이크로벤치마크가 아니라 빌드 파이프라인 결과로 보세요. 모든 `.css.ts`는 full Boa evaluator로 처리하며, 첫 모듈에 약 20ms, 이후 모듈마다 1ms 미만이 더 듭니다. CI는 Devup UI 행을 체크인된 기준값과 비교해 측정된 노이즈를 넘는 회귀에서 실패합니다(규칙은 `benchmark-gate.js`).
<!-- benchmark:end -->

## 작동 원리

Devup UI는 빌드 타임에 컴포넌트를 변환합니다. 클래스명은 CSS 크기를 최소화하기 위해 압축된 base-37 인코딩을 사용하여 생성됩니다.

**기본 변환:**

생성된 클래스명과 CSS 변수명은 버전, 빌드 상태, 접두사 설정에 따라 달라질 수 있으므로 직접 재사용하지 마세요.

개발자가 작성:

```tsx
import { Box } from '@devup-ui/react'

const example = <Box _hover={{ bg: 'blue' }} bg="red" p={4} />
```

Devup UI가 생성:

```tsx
import "@devup-ui/react/devup-ui.css";
const example = <div className="a b c" />;
```

CSS:

```css
/*! devup-ui v1.0.82 | Apache License 2.0 | https://devup-ui.com */.b{background:red}.c{padding:16px}.a:hover{background:blue}
```

**동적 값은 CSS 변수로 변환:**

개발자가 작성:

```tsx
import { Box } from '@devup-ui/react'

const example = <Box bg={colorVariable} />
```

Devup UI가 생성:

```tsx
import "@devup-ui/react/devup-ui.css";
const example = <div className="a" style={{ "--b": colorVariable }} />;
```

CSS:

```css
/*! devup-ui v1.0.82 | Apache License 2.0 | https://devup-ui.com */.a{background:var(--b)}
```

**복잡한 표현식과 반응형 배열:**

개발자가 작성:

```tsx
import { Box } from '@devup-ui/react'

const example = <Box bg={['red', 'blue', isActive ? 'green' : dynamicColor]} />
```

Devup UI가 생성:

```tsx
import "@devup-ui/react/devup-ui.css";
const example = <div className={`a b ${isActive ? "c" : "d"}`} style={{ "--e": dynamicColor }} />;
```

각 브레이크포인트에 대한 반응형 CSS:

```css
/*! devup-ui v1.0.82 | Apache License 2.0 | https://devup-ui.com */.a{background:red}@media(min-width:480px){.b{background:blue}}@media(min-width:768px){.c{background:green}.d{background:var(--e)}}
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
const boxExample = <Box typography="heading" />
```

**사용자 정의 shorthand:**

Shorthand는 디자인 토큰이 아니므로 `devup.json`이 아니라 빌드 플러그인
옵션에서 정의합니다. 각 shorthand는 하나의 prop 값을 여러 CSS 속성에
동일하게 적용합니다.

```ts
// vite.config.ts
export default defineConfig({
  plugins: [
    DevupUI({
      shorthands: {
        insetX: ['left', 'right'],
        scrollMarginX: ['scrollMarginLeft', 'scrollMarginRight'],
      },
    }),
  ],
})
```

플러그인이 실행되면 기본 경로인 `df/theme.d.ts`에 선언이 생성되어 일반
prop, 반응형 배열, 가상 선택자, 사용자 정의 selector에서 이름과 값 타입이
자동 완성됩니다. 설정을 바꾼 뒤에는 빌드 도구를 재시작해야 합니다.
`tsconfig.json`의 `include`가 `src`로 제한되어 있다면 `df/*.d.ts`도
포함하세요. `distDir`를 바꿨다면 해당 경로를 포함하면 됩니다.

```tsx
const pinned = <Box insetX={0} />
const responsive = <Box insetX={[0, null, 'auto']} />
const interactive = <Box _hover={{ insetX: 4 }} />
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
