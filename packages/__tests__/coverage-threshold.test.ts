import assert from 'node:assert/strict'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

const root = resolve(import.meta.dir, '../..')
const bunfig = await Bun.file(resolve(root, 'bunfig.toml')).text()
const manifest: unknown = await Bun.file(resolve(root, 'package.json')).json()
assert(
  typeof manifest === 'object' && manifest !== null && 'scripts' in manifest,
)
const { scripts: scriptValues } = manifest
assert(typeof scriptValues === 'object' && scriptValues !== null)
const scripts = scriptValues

function script(name: string): string {
  const command: unknown = Reflect.get(scripts, name)
  assert(typeof command === 'string', `package.json has no script ${name}`)
  return command
}

function bunTestCommands(
  name: string,
  visited = new Set<string>(),
): string[][] {
  if (visited.has(name)) return []
  visited.add(name)
  return script(name)
    .split(' && ')
    .map((step) => step.trim().split(/\s+/))
    .filter((words) => words[0] === 'bun')
    .flatMap((words) => {
      if (words[1] === 'run') return bunTestCommands(String(words[2]), visited)
      return words.includes('test') ? [words] : []
    })
}

function configPath(words: readonly string[]): string | undefined {
  for (const [index, word] of words.entries()) {
    if (word.startsWith('--config=')) return word.slice('--config='.length)
    if (word === '--config') {
      const path = words[index + 1]
      if (typeof path === 'string') return path
    }
  }
  return undefined
}

// `root` and `preload` name repository files; every other option applies unchanged.
const configs = [
  ['root', bunfig],
  ['vite', await Bun.file(resolve(root, 'bunfig.vite.toml')).text()],
] as const
const fixtureTest = `import { expect, test } from 'bun:test'
import { pick } from './source'

test('pick', () => {
  expect(pick(true)).toBe(1)
})
`
const fullyCovered =
  'export function pick(value: boolean) {\n  return value ? 1 : 2\n}\n'

function coverageRun(source: string, lane: 'root' | 'vite') {
  const directory = mkdtempSync(join(tmpdir(), 'coverage-threshold-'))
  try {
    const fixtureConfig = configs.find(([name]) => name === lane)?.[1]
    assert(typeof fixtureConfig === 'string')
    writeFileSync(
      join(directory, 'bunfig.toml'),
      fixtureConfig.replace(/^(?:root|preload) = .*$/gm, ''),
    )
    writeFileSync(join(directory, 'source.ts'), source)
    writeFileSync(join(directory, 'source.test.ts'), fixtureTest)
    const run = Bun.spawnSync(
      [
        process.execPath,
        'test',
        ...(lane === 'root' ? ['--isolate'] : []),
        '--coverage',
      ],
      {
        cwd: directory,
      },
    )
    return {
      exitCode: run.exitCode,
      output: run.stdout.toString() + run.stderr.toString(),
    }
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
}

it('requires every file to be fully covered', () => {
  expect(Bun.TOML.parse(bunfig)).toMatchObject({
    test: { coverageThreshold: { lines: 1, functions: 1, statements: 1 } },
  })
})

it('collects coverage in every Bun test run of the test script', () => {
  const commands = bunTestCommands('test')
  expect(commands).not.toEqual([])
  expect(commands.filter((words) => !words.includes('--coverage'))).toEqual([])
})

it('keeps the actual root native command serial and isolated and Vite neutral', () => {
  const commands = bunTestCommands('test')
  const roots = commands.filter((words) => configPath(words) === undefined)
  const vite = commands.filter((words) => configPath(words) !== undefined)
  expect(roots).toHaveLength(1)
  expect(vite).toHaveLength(1)
  expect(roots[0]).toContain('--isolate')
  expect(commands.some((words) => words.includes('--parallel'))).toBe(false)
  expect(vite[0]).not.toContain('--isolate')
})

it.each([
  ['--config=PATH', ['bun', '--config=./bunfig.vite.toml', 'test']],
  ['--config PATH', ['bun', '--config', './bunfig.vite.toml', 'test']],
])('finds explicit config paths written as %s', (_, words) => {
  expect(configPath(words)).toBe('./bunfig.vite.toml')
})

it('requires every explicit Bun test config to be fully covered', async () => {
  const paths = bunTestCommands('test')
    .map(configPath)
    .filter((path): path is string => path !== undefined)
  expect(paths).not.toEqual([])

  for (const path of paths) {
    const config = Bun.TOML.parse(await Bun.file(resolve(root, path)).text())
    expect(config).toMatchObject({
      test: { coverageThreshold: { lines: 1, functions: 1, statements: 1 } },
    })
  }
})

it.each(
  configs.flatMap(
    ([lane]) =>
      [
        [
          lane,
          'an uncovered line',
          'export function pick(value: boolean) {\n  if (value) {\n    return 1\n  }\n  return 2\n}\n',
        ],
        [
          lane,
          'an uncovered function',
          `${fullyCovered}export const unused = () => 3\n`,
        ],
      ] as const,
  ),
)('fails the %s run when a file has %s', (lane, _, source) => {
  const result = coverageRun(source, lane)
  expect(result).toMatchObject({ exitCode: 1 })
  expect(result.output).toMatch(/^\s*1 pass$/m)
  expect(result.output).toMatch(/^\s*0 fail$/m)
})

it.each(['root', 'vite'] as const)(
  'passes the %s run when every file is fully covered',
  (lane) => {
    expect(coverageRun(fullyCovered, lane)).toMatchObject({ exitCode: 0 })
  },
)
