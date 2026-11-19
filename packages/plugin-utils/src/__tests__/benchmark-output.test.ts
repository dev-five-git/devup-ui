import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import { evaluate } from '../../../../benchmark-gate.js'
import {
  BYTES_DEFINITION,
  isDeployed,
  measureOutput,
} from '../../../../benchmark-output.js'

describe('deployable build output', () => {
  it('names the definition runs, baselines and history record', () => {
    expect(BYTES_DEFINITION).toBe('deployable')
  })

  it.each([
    'BUILD_ID',
    'package.json',
    'required-server-files.json',
    'routes-manifest.json',
    'prerender-manifest.json',
    'server/app/page.js',
    'server/app/index.html',
    'server/app/index.rsc',
    'server/chunks/ssr/[turbopack]_runtime.js',
    'server/pages-manifest.json',
    'static/chunks/app.js',
    'static/css/page.css',
    'static/media/favicon.ico',
    'static/BUILD/_buildManifest.js',
    // Only the top-level directories are build-only
    'server/app/cache/page.js',
    'server/app/types/page.js',
    'static/chunks/build/app.js',
    'server/app/trace/page.js',
    'tracer.json',
  ])('deploys Next %s', (path) => {
    expect(isDeployed('next', path)).toBe(true)
  })

  it.each([
    'cache/.tsbuildinfo',
    'cache/webpack/client-production/0.pack',
    'cache/swc/plugins/v7/x.wasm',
    'cache/images/a/b.webp',
    'cache/devup-ui_build/page.css',
    'build/chunks/[turbopack]_runtime.js',
    'types/routes.d.ts',
    'types/validator.ts',
    'diagnostics/build-diagnostics.json',
    'trace',
    'trace-build',
    'trace-turbopack',
    'next-server.js.nft.json',
    'next-minimal-server.js.nft.json',
    'server/app/page.js.nft.json',
    'server/app/page.js.map',
    'server/chunks/ssr/[root-of-the-server]__0e._.js.map',
    'static/chunks/app.js.map',
    'tsconfig.tsbuildinfo',
  ])('leaves Next %s out', (path) => {
    expect(isDeployed('next', path)).toBe(false)
  })

  it.each([
    'client/_next/static/chunks/app.js',
    'client/_next/static/css/page.css',
    'client/.vite/manifest.json',
    'client/vinext-client-entry-manifest.json',
    'server/index.js',
    'server/ssr/index.js',
    'server/BUILD_ID',
    // Next's top-level build directories mean nothing in dist
    'cache/page.js',
    'types/page.js',
    'build/page.js',
    'trace',
  ])('deploys vinext dist/%s', (path) => {
    expect(isDeployed('vinext', path)).toBe(true)
  })

  it.each([
    'client/_next/static/chunks/app.js.map',
    'server/index.js.map',
    'tsconfig.tsbuildinfo',
  ])('leaves vinext dist/%s out', (path) => {
    expect(isDeployed('vinext', path)).toBe(false)
  })
})

describe('benchmark output measurement', () => {
  let directory: string

  beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'devup-benchmark-output-'))
  })

  afterEach(() => {
    rmSync(directory, { recursive: true, force: true })
  })

  function file(path: string, bytes: number) {
    const destination = join(directory, path)
    mkdirSync(dirname(destination), { recursive: true })
    writeFileSync(destination, Buffer.alloc(bytes, 'x'))
  }

  it.each(['static/css/page.css', 'static/chunks/nested/page.css'])(
    'counts only shipped CSS when Next emits %s alongside compiler caches',
    (path) => {
      file('.next/' + path, 494)
      file('.next/cache/devup-ui_build/page.css', 450)
      file('.next/cache/devup-ui_build/base.css', 68)
      file('.next/server/app/page.css', 83)
      file('.next/static/chunks/app.js', 101)

      const result = measureOutput(directory)

      expect(result).toEqual({ bytes: 678, cssBytes: 494, clientBytes: 595 })
    },
  )

  it('counts only the deployable Next output in bytes', () => {
    // Deployed: what `next start` serves
    file('.next/BUILD_ID', 1)
    file('.next/routes-manifest.json', 2)
    file('.next/server/app/page.js', 4)
    file('.next/server/app/index.html', 8)
    file('.next/static/chunks/app.js', 16)
    file('.next/static/css/page.css', 32)
    // Build-only: caches, type check, traces, source maps
    file('.next/cache/.tsbuildinfo', 1000)
    file('.next/cache/webpack/client-production/0.pack', 1000)
    file('.next/build/chunks/postcss.js', 1000)
    file('.next/types/routes.d.ts', 1000)
    file('.next/diagnostics/framework.json', 1000)
    file('.next/trace', 1000)
    file('.next/trace-build', 1000)
    file('.next/next-server.js.nft.json', 1000)
    file('.next/server/app/page.js.nft.json', 1000)
    file('.next/server/chunks/ssr/page.js.map', 1000)
    file('.next/static/chunks/app.js.map', 1000)

    const result = measureOutput(directory)

    expect(result).toEqual({ bytes: 63, cssBytes: 32, clientBytes: 48 })
  })

  it('prefers dist and counts client CSS when vinext also emits a Next stub', () => {
    file('.next/static/css/stub.css', 999)
    file('.next/cache/.tsbuildinfo', 999)
    file('dist/client/assets/page.css', 31)
    file('dist/client/assets/nested/shared.css', 17)
    file('dist/client/assets/app.js', 101)
    file('dist/client/assets/app.js.map', 999)
    file('dist/server/assets/page.css', 67)
    file('dist/server/app.js', 83)

    const result = measureOutput(directory)

    expect(result).toEqual({ bytes: 299, cssBytes: 48, clientBytes: 149 })
  })

  it('rejects measurement when neither output directory exists', () => {
    expect(() => measureOutput(directory)).toThrow()
  })

  it.each(['.next', 'dist'])(
    'rejects measurement when %s has no required browser-assets directory',
    (output) => {
      file(output + '/server/page.css', 123)

      expect(() => measureOutput(directory)).toThrow()
    },
  )

  it('does not fall back to Next when the preferred dist lacks client assets', () => {
    file('.next/static/css/page.css', 494)
    file('dist/server/page.css', 123)

    expect(() => measureOutput(directory)).toThrow()
  })

  it.each(['.next/static', 'dist/client'])(
    'counts zero CSS when the existing browser-assets directory %s is empty',
    (browser) => {
      mkdirSync(join(directory, browser), { recursive: true })

      const result = measureOutput(directory)

      expect(result).toEqual({ bytes: 0, cssBytes: 0, clientBytes: 0 })
    },
  )

  it.each(['.next/static', 'dist/client'])(
    'counts zero CSS when %s contains only non-CSS assets',
    (browser) => {
      file(browser + '/assets/app.js', 41)
      file(browser + '/assets/page.css.map', 23)

      const result = measureOutput(directory)

      // The source map is not deployed
      expect(result).toEqual({ bytes: 41, cssBytes: 0, clientBytes: 41 })
    },
  )

  it.each([
    { cssBytes: 494, failed: false },
    { cssBytes: 526, failed: false },
    { cssBytes: 527, failed: true },
  ])(
    'enforces shipped CSS headroom when the asset is $cssBytes bytes',
    ({ cssBytes, failed }) => {
      file('.next/static/css/page.css', cssBytes)
      file('.next/cache/devup-ui_build/page.css', 450)
      const baseline = {
        samples: 3,
        rows: {
          'devup-ui': {
            control: 'tailwind',
            ratio: 1,
            timeTolerance: 0.1,
            bytes: 1000,
            cssBytes: 494,
          },
        },
      }
      const run = {
        samples: 3,
        rows: {
          tailwind: { median: 1 },
          'devup-ui': { median: 1, ...measureOutput(directory) },
        },
      }

      const evaluation = evaluate(run, baseline)

      expect(evaluation.failed).toBe(failed)
      expect(evaluation.results).toEqual([
        {
          target: 'devup-ui',
          failures: failed
            ? ['CSS is 527 bytes, over the limit 526 (baseline 494)']
            : [],
        },
      ])
    },
  )
})

describe('approved CSS baseline headroom', () => {
  it.each([
    {
      target: 'devup-ui-single',
      baselineCss: 395,
      cssBytes: 427,
      failed: false,
    },
    {
      target: 'devup-ui-single',
      baselineCss: 395,
      cssBytes: 428,
      failed: true,
    },
    {
      target: 'vinext-devup-ui',
      baselineCss: 14873,
      cssBytes: 15616,
      failed: false,
    },
    {
      target: 'vinext-devup-ui',
      baselineCss: 14873,
      cssBytes: 15617,
      failed: true,
    },
  ])(
    'enforces the unchanged CSS gate for $target at $cssBytes bytes',
    ({ target, baselineCss, cssBytes, failed }) => {
      const baseline = {
        samples: 3,
        rows: {
          [target]: {
            control: 'tailwind',
            ratio: 1,
            timeTolerance: 0.1,
            bytes: 1000,
            cssBytes: baselineCss,
          },
        },
      }
      const run = {
        samples: 3,
        rows: {
          tailwind: { median: 1 },
          [target]: { median: 1, bytes: 1000, cssBytes },
        },
      }

      const evaluation = evaluate(run, baseline)

      expect(evaluation.failed).toBe(failed)
      expect(evaluation.results[0]?.failures).toHaveLength(failed ? 1 : 0)
    },
  )
})
