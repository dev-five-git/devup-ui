import { createHash } from 'node:crypto'
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, extname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

export const root = process.env.W72_ROOT ?? process.cwd()
export const out =
  process.env.W72_OUT ?? dirname(fileURLToPath(import.meta.url))
export const hash = (text) => createHash('sha256').update(text).digest('hex')
export const aliases = {
  '@emotion/react': null,
  '@emotion/styled': 'styled',
  '@emotion/css': null,
  'styled-components': 'styled',
  '@vanilla-extract/css': null,
}
export const theme = JSON.parse(
  readFileSync(join(root, 'apps/landing/devup.json'), 'utf8'),
).theme

export function files(dir) {
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(dir, entry.name)
      return entry.isDirectory() ? files(path) : [path]
    })
    .sort()
}

export function makeCorpus() {
  const corpus = []
  for (const [dir, group] of [
    ['apps/landing/src', 'landing'],
    ['packages/components/src', 'components'],
  ]) {
    for (const path of files(join(root, dir))) {
      if (!/\.[cm]?[jt]sx?$/.test(path)) continue
      corpus.push({
        filename: path.slice(root.length + 1).replaceAll('\\', '/'),
        group,
        code: readFileSync(path, 'utf8'),
      })
    }
  }
  const fixtures = []
  for (const path of files(join(root, 'libs/extractor'))) {
    if (extname(path) !== '.rs') continue
    const source = readFileSync(path, 'utf8')
    for (const match of source.matchAll(/r(#{0,6})"([\s\S]*?)"\1/g)) {
      if (/import\s[\s\S]*?from\s*['"]@devup-ui\/react/.test(match[2])) {
        fixtures.push({
          filename: `W72-fixture-${hash(match[2]).slice(0, 12)}.tsx`,
          group: 'fixtures',
          code: match[2],
          origin: path.slice(root.length + 1).replaceAll('\\', '/'),
        })
      }
    }
  }
  fixtures.sort(
    (a, b) =>
      b.code.length - a.code.length || a.filename.localeCompare(b.filename),
  )
  corpus.push(
    ...Array.from(
      new Map(fixtures.map((file) => [file.filename, file])).values(),
    ).slice(0, 8),
  )
  const elements = Array.from(
    { length: 5000 },
    (_, i) =>
      `<Box w="${i + 1}px" p={${i % 8}} bg="${i % 2 ? 'red' : 'blue'}" _hover={{color:'white'}} />`,
  ).join('\n')
  corpus.push({
    filename: 'W72-synthetic-5000.tsx',
    group: 'synthetic',
    code: `import {Box} from '@devup-ui/react';export const Page=()=> <>${elements}</>;`,
  })
  writeFileSync(join(out, 'W72-corpus.json'), JSON.stringify(corpus))
  const manifest = corpus.map(({ code, ...file }) => ({
    ...file,
    bytes: Buffer.byteLength(code),
    sha256: hash(code),
  }))
  writeFileSync(
    join(out, 'W72-corpus-manifest.json'),
    JSON.stringify(manifest, null, 2),
  )
  return corpus
}

export async function includeMdx() {
  const require = createRequire(join(root, 'apps/landing/package.json'))
  const mdxRequire = createRequire(require.resolve('@mdx-js/rollup'))
  const { compileSync } = await import(
    pathToFileURL(mdxRequire.resolve('@mdx-js/mdx')).href
  )
  const data = makeCorpus()
  for (const path of files(join(root, 'apps/landing/src'))) {
    if (extname(path) !== '.mdx') continue
    const source = readFileSync(path, 'utf8')
    data.push({
      filename: `${path.slice(root.length + 1).replaceAll('\\', '/')}.tsx`,
      group: 'landing-mdx',
      sourceSha256: hash(source),
      code: String(compileSync(source, { jsx: true })),
    })
  }
  writeFileSync(join(out, 'W72-corpus.json'), JSON.stringify(data))
  writeFileSync(
    join(out, 'W72-corpus-manifest.json'),
    JSON.stringify(
      data.map(({ code, ...file }) => ({
        ...file,
        bytes: Buffer.byteLength(code),
        sha256: hash(code),
      })),
      null,
      2,
    ),
  )
  return data
}

export function corpus() {
  const path = join(out, 'W72-corpus.json')
  return existsSync(path)
    ? JSON.parse(readFileSync(path, 'utf8'))
    : makeCorpus()
}

export function reset(binding) {
  binding.resetBuildState()
  binding.setDebug(false)
  binding.registerTheme(theme ?? {})
  binding.registerShorthands({})
}

export function extract(binding, file, mode = 'map', single = true) {
  const fn =
    mode === 'map' ? binding.codeExtract : binding.codeExtractWithoutSourceMap
  return fn(
    file.filename,
    file.code,
    '@devup-ui/react',
    'W72-css',
    single,
    false,
    false,
    aliases,
  )
}

export function output(result) {
  const value = {
    code: result.code,
    css: result.css ?? null,
    cssFile: result.cssFile ?? null,
    map: result.map ?? null,
    dependencies: result.dependencies,
    updatedBaseStyle: result.updatedBaseStyle,
  }
  result.free()
  return value
}

export function load(variant) {
  const path = join(
    out,
    `W72-${variant}`,
    variant.startsWith('N') ? 'index.cjs' : 'index.js',
  )
  return import(pathToFileURL(path).href).then(
    (module) => module.default ?? module,
  )
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const data = await includeMdx()
  process.stdout.write(
    JSON.stringify({
      count: data.length,
      groups: Object.fromEntries(
        ['landing', 'landing-mdx', 'components', 'fixtures', 'synthetic'].map(
          (group) => [
            group,
            data.filter((file) => file.group === group).length,
          ],
        ),
      ),
      bytes: data.reduce(
        (total, file) => total + Buffer.byteLength(file.code),
        0,
      ),
    }) + '\n',
  )
}
