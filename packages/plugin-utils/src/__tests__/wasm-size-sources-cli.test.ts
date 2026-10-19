import { spawnSync } from 'node:child_process'
import { readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { afterEach, describe, expect, it } from 'bun:test'

import { cli } from '../../../../wasm-size-sources.js'
import { sourceFixture } from './wasm-size-sources-fixture'

const roots: string[] = []
const resolver = fileURLToPath(
  new URL('../../../../resolve-wasm-size-sources.js', import.meta.url),
)

afterEach(() => {
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true })
})

function fixture(environment = process.env) {
  const input = sourceFixture(roots, 'Source fixture', environment)
  const githubEnvironment = {
    GITHUB_EVENT_NAME: 'pull_request',
    GITHUB_SHA: input.candidate,
    GITHUB_WORKSPACE: input.root,
    GITHUB_EVENT_PATH: join(input.root, 'event.json'),
    GITHUB_ENV: join(input.root, 'environment'),
    GITHUB_STEP_SUMMARY: join(input.root, 'summary.md'),
  }
  const event = { pull_request: { base: { sha: input.lagging } } }
  writeFileSync(githubEnvironment.GITHUB_EVENT_PATH, JSON.stringify(event))
  writeFileSync(githubEnvironment.GITHUB_ENV, 'EXISTING=1\n')
  writeFileSync(githubEnvironment.GITHUB_STEP_SUMMARY, 'previous\n')
  return { ...input, environment: githubEnvironment }
}

describe('WASM size source resolver CLI', () => {
  it.each([
    'GIT_DIR',
    'GIT_COMMON_DIR',
    'GIT_WORK_TREE',
    'GIT_INDEX_FILE',
    'GIT_OBJECT_DIRECTORY',
    'GIT_ALTERNATE_OBJECT_DIRECTORIES',
    'all',
  ] as const)('honors the workspace when inheriting foreign %s', (name) => {
    // Different authors produce genuinely different commit objects in each repo.
    const foreign = sourceFixture(roots, 'Foreign source fixture')
    const foreignEnvironment = {
      GIT_DIR: join(foreign.root, '.git'),
      GIT_COMMON_DIR: join(foreign.root, '.git'),
      GIT_WORK_TREE: foreign.root,
      GIT_INDEX_FILE: join(foreign.root, '.git', 'index'),
      GIT_OBJECT_DIRECTORY: join(foreign.root, '.git', 'objects'),
      GIT_ALTERNATE_OBJECT_DIRECTORIES: join(foreign.root, '.git', 'objects'),
    }
    const input = fixture({ ...process.env, ...foreignEnvironment })
    expect(foreign.candidate).not.toBe(input.candidate)

    const result = spawnSync('node', [resolver], {
      cwd: foreign.root,
      env: {
        ...process.env,
        ...input.environment,
        ...(name === 'all'
          ? foreignEnvironment
          : { [name]: foreignEnvironment[name] }),
      },
      encoding: 'utf8',
    })

    expect(result.status).toBe(0)
    expect(result.stderr).toBe('')
    expect(result.stdout).toBe(
      `BASE_SHA=${input.base}\nCANDIDATE_SHA=${input.candidate}\n`,
    )
    expect(readFileSync(input.environment.GITHUB_ENV, 'utf8')).toBe(
      `EXISTING=1\nBASE_SHA=${input.base}\nCANDIDATE_SHA=${input.candidate}\n`,
    )
  })

  it('rejects a commit found only in an inherited foreign alternate object store', () => {
    const input = fixture()
    const foreign = sourceFixture(roots, 'Foreign source fixture')

    const result = spawnSync('node', [resolver], {
      env: {
        ...process.env,
        ...input.environment,
        GITHUB_SHA: foreign.candidate,
        GIT_ALTERNATE_OBJECT_DIRECTORIES: join(foreign.root, '.git', 'objects'),
      },
      encoding: 'utf8',
    })

    expect(result.status).toBe(1)
    expect(result.stdout).toBe('')
    expect(readFileSync(input.environment.GITHUB_ENV, 'utf8')).toBe(
      'EXISTING=1\n',
    )
  })

  it('appends authenticated sources before builds when called in-process', () => {
    const input = fixture()

    const result = cli(input.environment)

    expect(result).toBe(0)
    expect(readFileSync(input.environment.GITHUB_ENV, 'utf8')).toBe(
      `EXISTING=1\nBASE_SHA=${input.base}\nCANDIDATE_SHA=${input.candidate}\n`,
    )
    const summary = readFileSync(input.environment.GITHUB_STEP_SUMMARY, 'utf8')
    expect(summary).toStartWith('previous\n')
    expect(summary).toContain(input.base)
    expect(summary).toContain(input.candidate)
  })

  it.each([
    'GITHUB_EVENT_PATH',
    'GITHUB_WORKSPACE',
    'GITHUB_ENV',
    'GITHUB_STEP_SUMMARY',
  ] as const)('rejects missing %s when called in-process', (name) => {
    const input = fixture()

    expect(() => cli({ ...input.environment, [name]: '' })).toThrow()
  })

  it.each(['pull_request', 'push'])(
    'emits exact source assignments when invoked for %s',
    (eventName) => {
      const input = fixture()
      const event = { before: input.base, after: input.candidate }
      writeFileSync(input.environment.GITHUB_EVENT_PATH, JSON.stringify(event))

      const result = spawnSync('node', [resolver], {
        env: {
          ...process.env,
          ...input.environment,
          GITHUB_EVENT_NAME: eventName,
        },
        encoding: 'utf8',
      })

      expect(result.status).toBe(0)
      expect(result.stdout).toBe(
        `BASE_SHA=${input.base}\nCANDIDATE_SHA=${input.candidate}\n`,
      )
      expect(result.stderr).toBe('')
      const summary = readFileSync(
        input.environment.GITHUB_STEP_SUMMARY,
        'utf8',
      )
      expect(summary).toContain(input.base)
      expect(summary).toContain(input.candidate)
    },
  )

  it.each(['root', 'missing', 'unsupported', 'json'] as const)(
    'reports visible failure without exporting sources when input is %s',
    (invalid) => {
      const input = fixture()
      const environment = { ...process.env, ...input.environment }
      switch (invalid) {
        case 'root':
          environment.GITHUB_SHA = input.lagging
          break
        case 'missing':
          environment.GITHUB_SHA = 'f'.repeat(40)
          break
        case 'unsupported':
          environment.GITHUB_EVENT_NAME = 'schedule'
          break
        case 'json':
          writeFileSync(environment.GITHUB_EVENT_PATH, '{')
          break
        default: {
          const exhaustive: never = invalid
          throw new Error(String(exhaustive))
        }
      }

      const result = spawnSync('node', [resolver], {
        env: environment,
        encoding: 'utf8',
      })

      expect(result.status).toBe(1)
      expect(result.stdout).toBe('')
      expect(result.stderr).not.toBe('')
      expect(readFileSync(environment.GITHUB_ENV, 'utf8')).toBe('EXISTING=1\n')
      expect(readFileSync(environment.GITHUB_STEP_SUMMARY, 'utf8')).toContain(
        '## WASM size: FAIL - source resolution',
      )
    },
  )
})
