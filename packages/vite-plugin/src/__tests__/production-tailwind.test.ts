import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { build } from 'vite'

import { DevupUI } from '../plugin'
import { nativeCompletions, productionFixture } from './production-fixture'

it('watches root-relative Tailwind imports and refreshes newly consumed CSS in production', async () => {
  // Given an unimported theme outside source roots and a nested custom config path.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/view.tsx',
    "import {Box} from '@devup-ui/react'; export const view=<Box className='bg-brand'/>;",
  )
  await fixture.file(
    'app/config/devup.json',
    '{"tailwind":{"css":"styles/tailwind.css"}}',
  )
  const tailwind = await fixture.file(
    'app/styles/tailwind.css',
    '@import "./brand.css";',
  )
  const brand = await fixture.file(
    'app/styles/brand.css',
    '@theme { --color-brand: #112233; }',
  )
  const nextBrand = await fixture.file(
    'app/styles/next.css',
    '@theme { --color-brand: #445566; }',
  )
  const registrations = new Set<string>()
  let restoreHooks = () => {}
  function observe<
    Context extends { addWatchFile(id: string): void },
    Args extends unknown[],
    Value,
  >(hook: (this: Context, ...args: Args) => Value) {
    return async function (
      this: Context,
      ...args: Args
    ): Promise<Awaited<Value>> {
      const original = this.addWatchFile
      const descriptor = Object.getOwnPropertyDescriptor(this, 'addWatchFile')
      this.addWatchFile = (id) => {
        const path = id.replaceAll('\\', '/')
        if ([tailwind, brand, nextBrand].includes(path)) registrations.add(path)
        return original.call(this, id)
      }
      try {
        return await hook.apply(this, args)
      } finally {
        if (descriptor) Object.defineProperty(this, 'addWatchFile', descriptor)
        else Reflect.deleteProperty(this, 'addWatchFile')
      }
    }
  }
  const outputs: {
    readonly css: string
    readonly code: string
    readonly files: readonly string[]
    readonly numbers: string
  }[] = []
  function expectAppliedColor(
    output: (typeof outputs)[number] | undefined,
    rulePattern: RegExp,
  ) {
    const rule = rulePattern.exec(output?.css ?? '')
    expect(rule?.[1]).toBeDefined()
    const classes = /className:\s*["']([^"']+)["']/
      .exec(output?.code ?? '')?.[1]
      ?.split(/\s+/)
    expect(classes).toContain(rule?.[1] ?? 'missing-new-color-class')
  }
  try {
    const watcher = await build({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      build: {
        watch: {},
        write: false,
        lib: { entry, formats: ['es'] },
        rolldownOptions: {
          external: ['react/jsx-runtime', 'react/jsx-dev-runtime'],
        },
      },
      plugins: [
        DevupUI({ singleCss: true, devupFile: 'config/devup.json' }),
        {
          name: 'observe-production-tailwind-output',
          configResolved(config) {
            const main = config.plugins.find(
              (plugin) => plugin.name === 'devup-ui',
            )
            const start = main?.buildStart
            const transform = main?.transform
            if (!main || !start || !transform)
              throw new TypeError('Expected native Devup watch hooks')
            main.buildStart =
              typeof start === 'function'
                ? observe(start)
                : { ...start, handler: observe(start.handler) }
            main.transform =
              typeof transform === 'function'
                ? observe(transform)
                : { ...transform, handler: observe(transform.handler) }
            restoreHooks = () => {
              main.buildStart = start
              main.transform = transform
            }
          },
          generateBundle: {
            order: 'post',
            handler(_options, bundle) {
              outputs.push({
                css: Object.values(bundle)
                  .filter((item) => item.type === 'asset')
                  .map((item) => item.source)
                  .join(''),
                code: Object.values(bundle)
                  .filter((item) => item.type === 'chunk')
                  .map((item) => item.code)
                  .join('\n'),
                files: [...registrations],
                numbers: wasm.exportFileMap(),
              })
              if (outputs.length > 3) outputs.shift()
            },
          },
        },
      ],
    })
    if (Array.isArray(watcher) || !('on' in watcher))
      throw new TypeError('Expected native Tailwind watcher')
    const next = nativeCompletions(watcher)
    try {
      await next()
      const first = outputs.at(-1)
      expect(first?.css).toMatch(/background(?:-color)?:#(?:112233|123)\b/)
      expectAppliedColor(
        first,
        /\.([\w-]+)\{background(?:-color)?:#(?:112233|123)\b[^}]*\}/,
      )
      expect(first?.files).toEqual(expect.arrayContaining([tailwind, brand]))
      // When only Tailwind CSS changes its local import, then the newly consumed file changes.
      await fixture.file('app/styles/tailwind.css', '@import "./next.css";')
      await next()
      const second = outputs.at(-1)
      expect(second?.css).toMatch(/background(?:-color)?:#(?:445566|456)\b/)
      expectAppliedColor(
        second,
        /\.([\w-]+)\{background(?:-color)?:#(?:445566|456)\b[^}]*\}/,
      )
      expect(second?.files).toContain(nextBrand)
      await fixture.file(
        'app/styles/next.css',
        '@theme { --color-brand: #778899; }',
      )
      await next()
      const third = outputs.at(-1)
      // Then actual emitted CSS refreshes without changing the generation's file numbering.
      expect(third?.css).toMatch(/background(?:-color)?:#(?:778899|789)\b/)
      expectAppliedColor(
        third,
        /\.([\w-]+)\{background(?:-color)?:#(?:778899|789)\b[^}]*\}/,
      )
      expect(second?.numbers).toBe(first?.numbers)
      expect(third?.numbers).toBe(first?.numbers)
    } finally {
      await watcher.close()
    }
  } finally {
    restoreHooks()
    await fixture.close()
  }
}, 60000)
