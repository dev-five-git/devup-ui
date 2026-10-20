// Writes the benchmark section of README.md and README_ko.md from one checked
// result file, so both languages publish the same run and the same numbers.
//
//   bun render-benchmark-readme.js          rewrite the sections
//   bun render-benchmark-readme.js --check  fail when a README differs from them
import { readFileSync, writeFileSync } from 'node:fs'

const START = '<!-- benchmark:start -->'
const END = '<!-- benchmark:end -->'

const results = JSON.parse(readFileSync('benchmark-results.json', 'utf8'))

const bytes = (value) => `${value.toLocaleString('en-US')} bytes`
const seconds = (value) => `${value.toFixed(2)}s`
const list = (samples) => samples.map(seconds).join(', ')

const TEXT = {
  en: {
    file: 'README.md',
    intro: ({ run }) =>
      `[Latest CI benchmark](${run.url}) (commit \`${run.commit}\`, Devup UI ${run.devupUi}) on \`${run.runner}\` with Next.js ${run.next}. All Next.js builds use the native TypeScript 7 CLI for type checking. The English and Korean tables are generated from the same checked result file, \`benchmark-results.json\`.`,
    sizes:
      'Build Size is every byte of the build output directory (`.next`, or `dist` for vinext); CSS Size is the emitted `.css` files alone, measured separately.',
    webpack: 'Webpack values are one cold build:',
    turbopack:
      'Turbopack values are medians of six cold builds in alternating order:',
    columns: {
      library: 'Library',
      version: 'Version',
      time: 'Build Time',
      median: 'Median Build Time',
      size: 'Build Size',
      css: 'CSS Size',
    },
    note: ({ turbopack }) => {
      const [tailwind, direct, stylesheet] = turbopack
      const range = (row) => [
        Math.min(...row.samples),
        Math.max(...row.samples),
      ]
      const overlap =
        range(tailwind)[0] <= range(direct)[1] &&
        range(direct)[0] <= range(tailwind)[1]
      const gap = tailwind.median - direct.median
      return `${overlap ? 'The Turbopack ranges overlap' : 'The Turbopack ranges do not overlap'}, so the direct-API median is ${Math.abs(gap).toFixed(2)}s (${Math.abs((gap / tailwind.median) * 100).toFixed(1)}%) ${gap >= 0 ? 'ahead of' : 'behind'} Tailwind on this fixture${overlap ? ' and the two are effectively at parity' : ''}. The six cold samples were Tailwind \`${list(tailwind.samples)}\`, direct Devup UI \`${list(direct.samples)}\`, and static \`.css.ts\` \`${list(stylesheet.samples)}\`. The fixtures have comparable app shapes, not pixel-identical styling: Tailwind styles the leading paragraph and button more heavily, while Devup UI exercises typed component/style props. Treat these as build-pipeline results rather than a per-rule microbenchmark. Every \`.css.ts\` module runs on the full Boa evaluator, which adds about 20 ms for the first module and under 1 ms for each further one.`
    },
  },
  ko: {
    file: 'README_ko.md',
    intro: ({ run }) =>
      `[최신 CI 벤치마크](${run.url}) (커밋 \`${run.commit}\`, Devup UI ${run.devupUi})는 \`${run.runner}\`에서 Next.js ${run.next}로 실행했습니다. 모든 Next.js 빌드는 네이티브 TypeScript 7 CLI로 타입 검사를 수행합니다. 영어와 한국어 표는 같은 결과 파일 \`benchmark-results.json\`에서 생성합니다.`,
    sizes:
      '빌드 사이즈는 빌드 출력 디렉터리(`.next`, vinext는 `dist`)의 모든 바이트이고, CSS 사이즈는 출력된 `.css` 파일만 따로 잰 값입니다.',
    webpack: 'Webpack 값은 cold build 1회 결과입니다.',
    turbopack:
      'Turbopack 값은 실행 순서를 번갈아 측정한 cold build 6회의 중앙값입니다.',
    columns: {
      library: '라이브러리',
      version: '버전',
      time: '빌드 시간',
      median: '빌드 시간 중앙값',
      size: '빌드 사이즈',
      css: 'CSS 사이즈',
    },
    note: ({ turbopack }) => {
      const [tailwind, direct, stylesheet] = turbopack
      const range = (row) => [
        Math.min(...row.samples),
        Math.max(...row.samples),
      ]
      const overlap =
        range(tailwind)[0] <= range(direct)[1] &&
        range(direct)[0] <= range(tailwind)[1]
      const gap = tailwind.median - direct.median
      return `Turbopack 측정 범위는 ${overlap ? '서로 겹치므로' : '겹치지 않으므로'} direct API 중앙값은 이 fixture에서 Tailwind보다 ${Math.abs(gap).toFixed(2)}s(${Math.abs((gap / tailwind.median) * 100).toFixed(1)}%) ${gap >= 0 ? '빠르며' : '느리며'}${overlap ? ' 사실상 동률입니다' : ''}. cold 샘플 6개는 Tailwind \`${list(tailwind.samples)}\`, direct Devup UI \`${list(direct.samples)}\`, static \`.css.ts\` \`${list(stylesheet.samples)}\`입니다. fixture는 앱 구조만 비슷하고 스타일이 픽셀 단위로 같지는 않습니다. Tailwind는 첫 문단과 버튼에 더 많은 스타일을 주고, Devup UI는 타입이 있는 컴포넌트/스타일 props를 사용합니다. 규칙 단위 마이크로벤치마크가 아니라 빌드 파이프라인 결과로 보세요. 모든 \`.css.ts\`는 full Boa evaluator로 처리하며, 첫 모듈에 약 20ms, 이후 모듈마다 1ms 미만이 더 듭니다.`
    },
  },
}

function table(text, rows, timeColumn, timeOf) {
  const columns = text.columns
  const header = [
    columns.library,
    columns.version,
    timeColumn,
    columns.size,
    columns.css,
  ]
  const body = rows.map((row) => {
    const bold = (value) => (row.devup ? `**${value}**` : value)
    return [
      bold(row.label),
      row.version,
      bold(seconds(timeOf(row))),
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
    text.intro(results),
    '',
    text.sizes,
    '',
    text.webpack,
    '',
    table(text, results.webpack, text.columns.time, (row) => row.seconds),
    '',
    text.turbopack,
    '',
    table(text, results.turbopack, text.columns.median, (row) => row.median),
    '',
    text.note(results),
    END,
  ].join('\n')
}

let stale = false
for (const text of Object.values(TEXT)) {
  const current = readFileSync(text.file, 'utf8').replaceAll('\r\n', '\n')
  const start = current.indexOf(START)
  const end = current.indexOf(END)
  if (start === -1 || end === -1)
    throw new Error(`${text.file} has no ${START} ... ${END} block`)
  const next =
    current.slice(0, start) + section(text) + current.slice(end + END.length)
  if (next === current) continue
  if (process.argv.includes('--check')) {
    console.error(
      `${text.file} differs from benchmark-results.json: run \`bun render-benchmark-readme.js\``,
    )
    stale = true
  } else {
    writeFileSync(text.file, next)
  }
}
if (stale) process.exit(1)
