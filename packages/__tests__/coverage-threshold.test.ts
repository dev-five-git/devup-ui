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
const { scripts } = manifest
assert(typeof scripts === 'object' && scripts !== null)

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

// `root` and `preload` name repository files; every other option applies unchanged.
const fixtureConfig = bunfig.replace(/^(?:root|preload) = .*$/gm, '')
const fixtureTest = `import { expect, test } from 'bun:test'
import { pick } from './source'

test('pick', () => {
  expect(pick(true)).toBe(1)
})
`
const fullyCovered =
  'export function pick(value: boolean) {\n  return value ? 1 : 2\n}\n'

function coverageRun(source: string) {
  const directory = mkdtempSync(join(tmpdir(), 'coverage-threshold-'))
  try {
    writeFileSync(join(directory, 'bunfig.toml'), fixtureConfig)
    writeFileSync(join(directory, 'source.ts'), source)
    writeFileSync(join(directory, 'source.test.ts'), fixtureTest)
    const run = Bun.spawnSync([process.execPath, 'test', '--coverage'], {
      cwd: directory,
    })
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

it.each([
  [
    'an uncovered line',
    'export function pick(value: boolean) {\n  if (value) {\n    return 1\n  }\n  return 2\n}\n',
  ],
  ['an uncovered function', `${fullyCovered}export const unused = () => 3\n`],
])('fails the run when a file has %s', (_, source) => {
  expect(coverageRun(source)).toMatchObject({ exitCode: 1 })
})

it('passes the run when every file is fully covered', () => {
  expect(coverageRun(fullyCovered)).toMatchObject({ exitCode: 0 })
})
