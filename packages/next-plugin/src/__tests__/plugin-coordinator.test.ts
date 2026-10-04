import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { basename, dirname, join } from 'node:path'

import { describe, expect, it } from 'bun:test'
import type { NextConfig } from 'next'

import { requestCoordinator } from '../coordinator-client'
import type { CoordinatorIdentity } from '../coordinator-port'
import { DevupUI } from '../plugin'
import { readCoordinatorState } from '../state'
import { box, installProjectHooks, makeProject } from './project'
import { installTurboHarness } from './turbo-harness'

installProjectHooks()
const harness = installTurboHarness()

interface LoaderOptions {
  coordinatorPortFile: string
  coordinatorIdentity: CoordinatorIdentity
  revisionFile?: string
}

function loaderOptions(config: NextConfig): LoaderOptions {
  const serialized = JSON.parse(JSON.stringify(config.turbopack?.rules))
  return serialized['*.{tsx,ts,jsx,js,mjs,mts,cts,cjs}'].loaders[0].options
}

function ask(
  options: LoaderOptions,
  path: string,
  extra: { method?: 'GET' | 'POST'; body?: string } = {},
) {
  return requestCoordinator({
    portFile: options.coordinatorPortFile,
    identity: options.coordinatorIdentity,
    resourcePath: path,
    path,
    timeoutMs: 20_000,
    ...extra,
  })
}

async function extract(options: LoaderOptions, root: string, file: string) {
  const resourcePath = join(root, file)
  return JSON.parse(
    await ask(options, '/extract', {
      method: 'POST',
      body: JSON.stringify({
        filename: file,
        code: readFileSync(resourcePath, 'utf-8'),
        resourcePath,
      }),
    }),
  ) as { code: string }
}

describe('the plugin with the real coordinator', () => {
  it('serves a complete production stylesheet and drains after the compile', async () => {
    harness.startRealCoordinators()
    const root = makeProject({
      'src/app/page.tsx': box('bg="red"'),
      'src/app/b/page.tsx': box('bg="blue"'),
      'src/dead/never.tsx': box('bg="green"'),
    })
    process.chdir(root)
    const config = DevupUI({}, { singleCss: true })
    const options = loaderOptions(config)

    const css = await ask(options, '/css?importMainCss=false&waitForIdle=true')
    const code = await extract(options, root, 'src/app/page.tsx')
    const again = await ask(
      options,
      '/css?importMainCss=false&waitForIdle=true',
    )

    expect(css).toContain('background:red')
    expect(css).toContain('background:blue')
    expect(css).not.toContain('background:green')
    expect(again).toBe(css)
    expect(code.code).toContain('devup-ui.css')
    expect(existsSync(options.coordinatorPortFile)).toBe(true)

    await config.compiler?.runAfterProductionCompile?.({
      projectDir: root,
      distDir: '.next',
    })

    expect(existsSync(options.coordinatorPortFile)).toBe(false)
    harness.handlers.exit![0]!()
    expect(existsSync(join(options.coordinatorPortFile, '..'))).toBe(false)
  })

  it('refuses a request that carries another app’s identity', async () => {
    harness.startRealCoordinators()
    process.chdir(makeProject({ 'src/app/page.tsx': box('bg="red"') }))
    const config = DevupUI({}, { singleCss: true })
    const options = loaderOptions(config)
    await ask(options, '/css?importMainCss=false&waitForIdle=true')

    await expect(
      ask(
        {
          ...options,
          coordinatorIdentity: {
            ...options.coordinatorIdentity,
            token: '00000000-0000-4000-8000-000000000000',
          },
        },
        '/css?importMainCss=false&waitForIdle=true',
      ),
    ).rejects.toThrow('identity mismatch')

    harness.handlers.exit![0]!()
  })

  it('keeps names across a development restart and drops what was deleted', async () => {
    harness.startRealCoordinators()
    process.env.NODE_ENV = 'development'
    const root = makeProject({
      'src/app/a/page.tsx': box('bg="red"'),
      'src/app/b/page.tsx': box('bg="blue"'),
    })
    process.chdir(root)
    const first = loaderOptions(DevupUI({}, { singleCss: true }))
    const firstCss = await ask(first, '/css?importMainCss=false')
    const [firstExit] = harness.handlers.exit!
    await harness.handlers.beforeExit![0]!()
    firstExit!()

    rmSync(join(root, 'src/app/b/page.tsx'))
    writeFileSync(join(root, 'src/app/c.tsx'), box('bg="green"'))
    const second = loaderOptions(DevupUI({}, { singleCss: true }))
    const secondCss = await ask(second, '/css?importMainCss=false')
    await extract(second, root, 'src/app/c.tsx')
    const laterCss = await ask(second, '/css?importMainCss=false')
    await harness.handlers.beforeExit![1]!()
    harness.handlers.exit![1]!()

    expect(firstCss).toContain('background:red')
    expect(firstCss).toContain('background:blue')
    expect(secondCss).toContain('background:red')
    expect(secondCss).not.toContain('background:blue')
    expect(secondCss).not.toContain('background:green')
    expect(laterCss).toContain('background:green')
    expect(second.coordinatorPortFile).not.toBe(first.coordinatorPortFile)
    expect(existsSync(first.coordinatorPortFile)).toBe(false)
    const appDir = dirname(dirname(dirname(second.coordinatorPortFile)))
    const snapshot = readCoordinatorState(
      join(appDir, 'snapshot.json'),
      basename(appDir),
    )
    expect(snapshot?.fileMap).toEqual({
      'src/app/a/page.tsx': 0,
      'src/app/b/page.tsx': 1,
      'src/app/c.tsx': 2,
    })
    expect(snapshot?.inputs.map((input) => input.filename)).toEqual([
      'src/app/a/page.tsx',
      'src/app/c.tsx',
    ])
  })
})
