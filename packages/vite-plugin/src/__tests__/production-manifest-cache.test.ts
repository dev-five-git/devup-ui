import * as fs from 'node:fs'
import { resolve } from 'node:path'

import { expect, it, mock, spyOn } from 'bun:test'
import { resolveConfig } from 'vite'

import * as scanner from '../../../plugin-utils/src/import-scanner'
import type { ProductionOptions } from '../production-manifest'

const sourceChild = process.env.DEVUP_MANIFEST_SOURCE === import.meta.path
if (sourceChild)
  mock.module(
    '@devup-ui/plugin-utils',
    () => import('../../../plugin-utils/src/index'),
  )
const { prepareProductionManifest } = await import('../production-manifest')
const { productionFixture } = await import('./production-fixture')

if (!sourceChild)
  it('counts real source execution in a disposable preparation process', async () => {
    const child = Bun.spawn(
      [process.execPath, 'test', import.meta.path, '--no-coverage'],
      {
        env: { ...process.env, DEVUP_MANIFEST_SOURCE: import.meta.path },
        stdout: 'pipe',
        stderr: 'pipe',
        timeout: 4000,
        killSignal: 'SIGKILL',
      },
    )
    const output = [
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ] as const
    try {
      const [code, stdout, stderr] = await Promise.all(output)
      expect({ code, stdout, stderr }).toMatchObject({ code: 0 })
      console.info(stdout)
    } finally {
      child.kill('SIGKILL')
      await Promise.allSettled(output)
    }
  })

const options: ProductionOptions = {
  package: '@devup-ui/react',
  devupFile: 'devup.json',
  distDir: 'df',
  cssDir: undefined,
  extractCss: true,
  debug: false,
  include: ['target'],
  singleCss: false,
  prefix: undefined,
  shorthands: undefined,
  sourceDirs: undefined,
  mdxExtensions: ['.mdx'],
  atomHoist: 2,
  importAliases: {},
}

it('reads each successful lexical input once across environments, closures and relevance while atom hoisting is deferred, then expires', async () => {
  // Given two routes, dormant roots, inherited config and two real export branches.
  const fixture = await productionFixture()
  const shared = await fixture.file(
    'app/src/shared.js',
    '/* @devup-ui/react */ export const shared=1;',
  )
  const a = await fixture.file(
    'app/src/a.js',
    "/* @devup-ui/react */ export * from './shared.js'; export * from 'palette';",
  )
  const b = await fixture.file(
    'app/src/b.js',
    "/* @devup-ui/react */ export * from './shared.js';",
  )
  const dormant = await fixture.file(
    'app/src/dormant.js',
    '/* @devup-ui/react */ export const dormant=1;',
  )
  const configPath = await fixture.file(
    'app/tsconfig.json',
    '{"extends":"./base.json"}',
  )
  const base = await fixture.file(
    'app/base.json',
    '{"compilerOptions":{"baseUrl":"."}}',
  )
  const packagePath = await fixture.file(
    'app/node_modules/palette/package.json',
    '{"name":"palette","exports":{"browser":"./browser.js","node":"./node.js"}}',
  )
  for (const branch of ['browser', 'node'])
    await fixture.file(
      `app/node_modules/palette/${branch}.js`,
      `export * from './${branch}/node_modules/target/index.js';`,
    )
  const browser = await fixture.file(
    'app/node_modules/palette/browser/node_modules/target/index.js',
    '/* @devup-ui/react */ export const branch="browser";',
  )
  const server = await fixture.file(
    'app/node_modules/palette/node/node_modules/target/index.js',
    '/* @devup-ui/react */ export const branch="node";',
  )
  const config = await resolveConfig(
    {
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      environments: {
        browser: { consumer: 'client', resolve: { conditions: ['browser'] } },
        server: { consumer: 'server', resolve: { conditions: ['node'] } },
      },
      build: { lib: { entry: [a, b], formats: ['es'] } },
    },
    'build',
  )
  const tracked = [
    a,
    b,
    shared,
    dormant,
    configPath,
    base,
    packagePath,
    browser,
    server,
  ].map((path) => resolve(path))
  const counts = new Map<string, number>()
  const scans = new Map<string, number>()
  const originalScan = scanner.scanImportRequests
  const scan = sourceChild ? spyOn(scanner, 'scanImportRequests') : undefined
  scan?.mockImplementation((source, jsx, typescript = true) => {
    const key = JSON.stringify([source, jsx, typescript])
    scans.set(key, (scans.get(key) ?? 0) + 1)
    return originalScan(source, jsx, typescript)
  })
  const original = fs.readFileSync
  const read = spyOn(fs, 'readFileSync').mockImplementation(
    new Proxy(original, {
      apply(target, receiver, args) {
        const result = Reflect.apply(target, receiver, args)
        const path = String(args[0])
        if (tracked.includes(path) && typeof result === 'string')
          counts.set(path, (counts.get(path) ?? 0) + 1)
        return result
      },
    }),
  )
  try {
    // When the real preparation visits all policies and environments.
    const manifest = await prepareProductionManifest(config, options, () => {})
    // Then membership stays context-local, atom activation stays deferred and only byte reads are shared.
    for (const [key, branch] of [
      ['browser', browser],
      ['server', server],
    ] as const) {
      const context = manifest.contexts.find(
        (context) => context.context === key,
      )
      expect(
        context?.files
          .map(({ path, realPath, id, context }) => [
            context,
            path.replaceAll('\\', '/'),
            realPath.replaceAll('\\', '/'),
            id,
          ])
          .sort(),
      ).toEqual(
        [a, b, shared, dormant, branch]
          .map((path) => [key, path, path, path])
          .sort(),
      )
      expect(context?.conditions).toEqual([
        'import',
        key === 'browser' ? 'browser' : 'node',
      ])
      expect(context?.routes).toEqual({})
      expect(context?.canonical).toEqual({})
      expect(context?.threshold).toBeUndefined()
    }
    expect(Object.fromEntries(counts)).toEqual(
      Object.fromEntries(tracked.map((path) => [path, 1])),
    )
    if (sourceChild) {
      expect(scans.size).toBeGreaterThan(0)
      expect([...scans.values()]).toEqual([...scans.keys()].map(() => 1))
      console.info(
        JSON.stringify({
          event: 'preparation-scanner-executions',
          tuples: [...scans],
        }),
      )
    }
    counts.clear()
    await fixture.file('app/src/dormant.js', 'export const dormant=2;')
    await fixture.file(
      'app/node_modules/palette/package.json',
      '{"name":"palette","exports":"./node.js"}',
    )
    const next = await prepareProductionManifest(config, options, () => {})
    expect(
      next.contexts
        .find((context) => context.context === 'browser')
        ?.scan.toSorted(),
    ).toEqual([a, b, shared, server].sort())
    expect(counts.get(resolve(packagePath))).toBe(1)
    expect(counts.get(resolve(dormant))).toBe(1)
  } finally {
    scan?.mockRestore()
    read.mockRestore()
    await fixture.close()
  }
})

it('discards a failed preparation epoch before repaired sources are prepared again', async () => {
  // Given a source whose selecting manifest cannot be parsed.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/main.js',
    "/* @devup-ui/react */ import 'palette';",
  )
  await fixture.file('app/node_modules/palette/package.json', '{')
  await fixture.file(
    'app/node_modules/palette/index.js',
    '/* @devup-ui/react */ export {};',
  )
  const config = await resolveConfig(
    {
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      build: { lib: { entry, formats: ['es'] } },
    },
    'build',
  )
  try {
    // When a failed invocation is followed by a separate repaired invocation.
    await expect(
      prepareProductionManifest(config, options, () => {}),
    ).rejects.toThrow('package.json')
    await fixture.file(
      'app/node_modules/palette/package.json',
      '{"name":"palette","main":"index.js"}',
    )
    const repaired = await prepareProductionManifest(config, options, () => {})
    // Then no successful raw read from the failed epoch contaminates the repair.
    expect(
      repaired.contexts.every((context) => context.scan.includes(entry)),
    ).toBe(true)
  } finally {
    await fixture.close()
  }
})
