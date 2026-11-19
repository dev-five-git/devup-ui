// Writes the benchmark section of README.md, README_ko.md and the package
// READMEs that npm publishes from one checked result file, so every copy shows
// the same run and the same numbers.
// Every row is the median of the same number of cold samples, shown with its
// range. CI refreshes the file after every merge to main (benchmark-refresh.js);
// `benchmark-check.js --write-results` does the same by hand.
//
//   bun render-benchmark-readme.js          rewrite the sections
//   bun render-benchmark-readme.js --check  fail when a README differs from them
import { readFileSync, writeFileSync } from 'node:fs'

import { bytesDefinition } from './benchmark-gate.js'

const START = '<!-- benchmark:start -->'
const END = '<!-- benchmark:end -->'

const results = JSON.parse(readFileSync('benchmark-results.json', 'utf8'))
const rowsOf = (table) => results.rows.filter((row) => row.table === table)

const bytes = (value) => `${value.toLocaleString('en-US')} bytes`
const seconds = (value) => `${value.toFixed(2)}s`
const range = (row) => `${seconds(row.min)} - ${seconds(row.max)}`
const list = (samples) => samples.map(seconds).join(', ')

// What Build Size counts follows the run the tables come from: the deployable
// output, or the whole output directory of runs recorded before that.
const DEFINITION =
  bytesDefinition(results.run) === 'deployable' ? 'deployable' : 'legacy'

const BUILD_SIZE = {
  en: {
    deployable:
      "Build Size is the deployable build output, uncompressed: what a deployment uploads, never the caches and type-check files a build leaves next to it. For Next.js it is `.next` without `cache/` (the Webpack/Turbopack and SWC caches and TypeScript's `.tsbuildinfo`), Turbopack's build-time `build/`, the generated `types/`, `diagnostics/`, the `trace` files, the output-tracing `*.nft.json` records and source maps, which is about what Next.js's `output: 'standalone'` copies plus `.next/static`; for vinext it is `dist` without source maps. The rule is `isDeployed` in `benchmark-output.js`; history entries recorded before it (no `bytesDefinition` in `benchmark/history.json`) counted the whole output directory.",
    legacy:
      'Build Size (incl. caches) is every byte of the build output directory (`.next`, or `dist` for vinext); most of it is the bundler cache and server output, and it moves by a few kilobytes between identical builds, so it is kept for continuity rather than as what users download.',
  },
  ko: {
    deployable:
      "빌드 사이즈는 배포되는 빌드 결과물의 압축 전 크기입니다. 빌드가 남기는 캐시와 타입 검사 파일은 배포하지 않으므로 세지 않습니다. Next.js는 `.next`에서 `cache/`(Webpack/Turbopack·SWC 캐시와 TypeScript `.tsbuildinfo`), Turbopack 빌드 전용 `build/`, 생성된 `types/`, `diagnostics/`, `trace` 파일, output tracing용 `*.nft.json`, 소스맵을 뺀 크기로, Next.js `output: 'standalone'`이 복사하는 파일에 `.next/static`을 더한 것과 거의 같습니다. vinext는 소스맵을 뺀 `dist`입니다. 규칙은 `benchmark-output.js`의 `isDeployed`이며, 그 전에 기록된 history 항목(`benchmark/history.json`에 `bytesDefinition`이 없는 항목)은 출력 디렉터리 전체를 셌습니다.",
    legacy:
      '빌드 사이즈(캐시 포함)는 빌드 출력 디렉터리(`.next`, vinext는 `dist`)의 모든 바이트입니다. 대부분이 번들러 캐시와 서버 출력이고 같은 빌드끼리도 수 KB씩 달라지므로, 사용자가 받는 크기가 아니라 이전 결과와의 연속성을 위해 남겨 둡니다.',
  },
}

const TEXT = {
  en: {
    files: [
      'README.md',
      ...[
        'react',
        'components',
        'reset-css',
        'next-plugin',
        'vite-plugin',
        'rsbuild-plugin',
        'webpack-plugin',
      ].map((name) => `packages/${name}/README.md`),
    ],
    intro: ({ run, samples }) =>
      `[Latest CI benchmark](${run.url}) (commit \`${run.commit}\`, Devup UI ${run.devupUi}, ${run.date.slice(0, 10)}) on \`${run.runner}\` with Next.js ${run.next}. Every row is the median of ${samples} cold builds that ran in rotating order, shown with the range of the samples. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, \`benchmark-results.json\`, which CI refreshes after every merge to main and records in \`benchmark/history.json\`, one entry per commit, so a slower build can be traced to the merge that caused it.`,
    sizes: (definition) =>
      'Client Size is the JavaScript and CSS shipped to the browser, uncompressed: `.next/static/**/*.{js,css}` or vinext `dist/client/**/*.{js,css}`. ' +
      BUILD_SIZE.en[definition] +
      " CSS Size counts only browser-shipped CSS: `.next/static/**/*.css` (Webpack `static/css` and Turbopack `static/chunks`) or vinext `dist/client/**/*.css`, excluding caches and server CSS. A missing browser-assets directory is an error; an existing directory with no CSS counts zero. Every size is measured on every sample and shown as the median; `benchmark-results.json` keeps the samples with their range, and rows measured before Client Size was recorded show n/a. Approved CSS measurement corrections are limited to explicitly named `devup-ui`, `devup-ui-single` and `vinext-devup-ui` rows. `bun benchmark-check.js --write-css-baseline devup-ui` retains the per-file mode; `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui` selects only the other two rows in one comma-separated argument. Only selected existing rows' CSS bytes change; unlike `--write-baseline`, this preserves sample count, build bytes, ratios, tolerances, all unselected rows and calibration history. Missing or unsupported names and conflicting writer modes are rejected.",
    webpack: 'Webpack:',
    turbopack: 'Turbopack:',
    columns: {
      library: 'Library',
      version: 'Version',
      time: 'Median Build Time',
      range: 'Range',
      client: 'Client Size',
      size: { deployable: 'Build Size', legacy: 'Build Size (incl. caches)' },
      css: 'CSS Size',
    },
    note: ({ samples }, [tailwind, direct, stylesheet]) => {
      const spread = (row) => [row.min, row.max]
      const overlap =
        spread(tailwind)[0] <= spread(direct)[1] &&
        spread(direct)[0] <= spread(tailwind)[1]
      const gap = tailwind.median - direct.median
      return `${overlap ? 'The Turbopack ranges overlap' : 'The Turbopack ranges do not overlap'}, so the direct-API median is ${Math.abs(gap).toFixed(2)}s (${Math.abs((gap / tailwind.median) * 100).toFixed(1)}%) ${gap >= 0 ? 'ahead of' : 'behind'} Tailwind on this fixture${overlap ? ' and the two are effectively at parity' : ''}. The ${samples} cold samples were Tailwind \`${list(tailwind.samples)}\`, direct Devup UI \`${list(direct.samples)}\`, and static \`.css.ts\` \`${list(stylesheet.samples)}\`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every \`.css.ts\` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one. CI compares the Devup UI rows with a checked baseline (see \`benchmark-gate.js\` for the rule) and fails on a regression beyond the measured noise.`
    },
  },
  ko: {
    files: ['README_ko.md'],
    intro: ({ run, samples }) =>
      `[최신 CI 벤치마크](${run.url}) (커밋 \`${run.commit}\`, Devup UI ${run.devupUi}, ${run.date.slice(0, 10)})는 \`${run.runner}\`에서 Next.js ${run.next}로 실행했습니다. 모든 행은 순서를 돌려 가며 측정한 cold build ${samples}회의 중앙값이며 샘플의 범위를 함께 보여줍니다. 모든 Next.js 빌드는 네이티브 TypeScript 7 CLI로 타입 검사를 수행합니다. 영어와 한국어 표는 같은 결과 파일 \`benchmark-results.json\`에서 생성합니다. CI는 main에 병합될 때마다 이 파일을 갱신하고 커밋마다 하나씩 \`benchmark/history.json\`에 기록하므로, 빌드가 느려지면 원인이 된 병합을 찾을 수 있습니다.`,
    sizes: (definition) =>
      '클라이언트 사이즈는 브라우저에 제공되는 JavaScript와 CSS의 압축 전 크기입니다: `.next/static/**/*.{js,css}` 또는 vinext `dist/client/**/*.{js,css}`. ' +
      BUILD_SIZE.ko[definition] +
      ' CSS 사이즈는 브라우저에 제공되는 CSS만 측정합니다: `.next/static/**/*.css`(Webpack `static/css`, Turbopack `static/chunks`) 또는 vinext `dist/client/**/*.css`. 캐시와 서버 CSS는 제외합니다. 브라우저 에셋 디렉터리가 없으면 오류이며, 존재하지만 CSS가 없으면 0입니다. 모든 사이즈는 샘플마다 측정해 중앙값으로 보여 주며, `benchmark-results.json`에는 샘플과 범위가 함께 남습니다. 클라이언트 사이즈를 기록하기 전에 측정한 행은 n/a로 표시합니다. 승인된 CSS 측정 보정은 명시적으로 지정한 `devup-ui`, `devup-ui-single`, `vinext-devup-ui` 행으로 제한됩니다. `bun benchmark-check.js --write-css-baseline devup-ui`는 기존 per-file 모드를 유지하며, `bun benchmark-check.js --write-css-baseline devup-ui-single,vinext-devup-ui`는 쉼표로 구분한 인자 하나로 나머지 두 행만 선택합니다. 선택한 기존 행의 CSS 바이트만 바꾸고, `--write-baseline`과 달리 샘플 수, 빌드 바이트, 비율, 허용 오차, 선택하지 않은 모든 행과 보정 이력을 보존합니다. 이름 누락, 지원하지 않는 이름, 다른 쓰기 모드와의 혼용은 오류입니다.',
    webpack: 'Webpack:',
    turbopack: 'Turbopack:',
    columns: {
      library: '라이브러리',
      version: '버전',
      time: '빌드 시간 중앙값',
      range: '범위',
      client: '클라이언트 사이즈',
      size: { deployable: '빌드 사이즈', legacy: '빌드 사이즈(캐시 포함)' },
      css: 'CSS 사이즈',
    },
    note: ({ samples }, [tailwind, direct, stylesheet]) => {
      const spread = (row) => [row.min, row.max]
      const overlap =
        spread(tailwind)[0] <= spread(direct)[1] &&
        spread(direct)[0] <= spread(tailwind)[1]
      const gap = tailwind.median - direct.median
      return `Turbopack 측정 범위는 ${overlap ? '서로 겹치므로' : '겹치지 않으므로'} direct API 중앙값은 이 fixture에서 Tailwind보다 ${Math.abs(gap).toFixed(2)}s(${Math.abs((gap / tailwind.median) * 100).toFixed(1)}%) ${gap >= 0 ? '빠르며' : '느리며'}${overlap ? ' 사실상 동률입니다' : ''}. cold 샘플 ${samples}개는 Tailwind \`${list(tailwind.samples)}\`, direct Devup UI \`${list(direct.samples)}\`, static \`.css.ts\` \`${list(stylesheet.samples)}\`입니다. fixture는 앱 구조만 비슷하고 스타일이 픽셀 단위로 같지는 않습니다. Tailwind는 첫 문단과 버튼에 더 많은 스타일을 주고, Devup UI는 타입이 있는 컴포넌트/스타일 props를 사용합니다. 규칙 단위 마이크로벤치마크가 아니라 빌드 파이프라인 결과로 보세요. 모든 \`.css.ts\`는 full Boa evaluator로 처리하며, 첫 모듈에 약 20ms, 이후 모듈마다 1ms 미만이 더 듭니다. CI는 Devup UI 행을 체크인된 기준값과 비교해 측정된 노이즈를 넘는 회귀에서 실패합니다(규칙은 \`benchmark-gate.js\`).`
    },
  },
}

function table(text, rows) {
  const columns = text.columns
  const header = [
    columns.library,
    columns.version,
    columns.time,
    columns.range,
    columns.client,
    columns.size[DEFINITION],
    columns.css,
  ]
  const body = rows.map((row) => {
    const bold = (value) => (row.devup ? `**${value}**` : value)
    return [
      bold(row.label),
      row.version,
      bold(seconds(row.median)),
      range(row),
      row.clientBytes === null ? 'n/a' : bytes(row.clientBytes),
      bytes(row.bytes),
      bytes(row.cssBytes),
    ]
  })
  const widths = header.map((_, column) =>
    Math.max(...[header, ...body].map((line) => [...line[column]].length)),
  )
  const line = (cells) =>
    `| ${cells.map((cell, column) => cell + ' '.repeat(widths[column] - [...cell].length)).join(' | ')} |`
  return [
    line(header),
    `| ${widths.map((width) => '-'.repeat(width)).join(' | ')} |`,
    ...body.map(line),
  ].join('\n')
}

function section(text) {
  return [
    START,
    '',
    text.intro(results),
    '',
    text.sizes(DEFINITION),
    '',
    text.webpack,
    '',
    table(text, rowsOf('webpack')),
    '',
    text.turbopack,
    '',
    table(text, rowsOf('turbopack')),
    '',
    text.note(results, rowsOf('turbopack')),
    END,
  ].join('\n')
}

let stale = false
for (const text of Object.values(TEXT)) {
  for (const file of text.files) {
    const current = readFileSync(file, 'utf8').replaceAll('\r\n', '\n')
    const start = current.indexOf(START)
    const end = current.indexOf(END)
    if (start === -1 || end === -1)
      throw new Error(`${file} has no ${START} ... ${END} block`)
    const next =
      current.slice(0, start) + section(text) + current.slice(end + END.length)
    if (next === current) continue
    if (process.argv.includes('--check')) {
      console.error(
        `${file} differs from benchmark-results.json: run \`bun render-benchmark-readme.js\``,
      )
      stale = true
    } else {
      writeFileSync(file, next)
    }
  }
}
if (stale) process.exit(1)
