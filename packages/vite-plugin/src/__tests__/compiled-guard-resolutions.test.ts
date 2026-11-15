import assert from 'node:assert/strict'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'
import type { Plugin } from 'vite'

import { createCompiledGuard } from '../compiled-guard'
import { productionFixture } from './production-fixture'

it('leaves forwarding unproven when the standalone guard has no valid observer', () => {
  // Given final imported IDs, but no installed native resolution observer.
  const guard = createCompiledGuard({
    package: '@devup-ui/react',
    mdxExtensions: [],
    importAliases: {},
    extractCss: true,
  })
  const modules = new Map([
    [
      '/entry.custom',
      {
        code: 'import {Widget} from "./barrel.js"; export const value=Widget;',
        importedIds: ['/barrel.js'],
      },
    ],
    [
      '/barrel.js',
      {
        code: 'export {Box as Widget} from "@devup-ui/react";',
        importedIds: ['@devup-ui/react'],
      },
    ],
  ])
  const hook = guard.buildEnd
  assert(typeof hook === 'function')
  // When the real guard verifies final facts without actual edge observations.
  // Then it neither fabricates forwarding authority nor performs a late resolve.
  expect(() =>
    Reflect.apply(
      hook,
      {
        getModuleIds: () => modules.keys(),
        getModuleInfo: (id: string) => modules.get(id) ?? null,
        resolve: () => assert.fail('Final verification must not resolve edges'),
        error: () => assert.fail('Unobserved forwarding must remain unproven'),
      },
      [],
    ),
  ).not.toThrow()
})

it.each([false, true])(
  'omits unused guard resolutions with identical native output when virtual=%s',
  async (virtual) => {
    // Given executable imports that are not forwarding export origins.
    const fixture = await productionFixture()
    const code =
      'import {read} from "./ordinary.js"; export const result=read()'
    await fixture.file(
      'app/src/main.js',
      virtual ? 'export {result} from "virtual-executable"' : code,
    )
    await fixture.file(
      'app/src/ordinary.js',
      'export function read(){return 42}',
    )
    const guard = createCompiledGuard({
      package: '@devup-ui/react',
      mdxExtensions: ['.mdx'],
      importAliases: {},
      extractCss: true,
    })
    const hook = guard.buildEnd
    if (typeof hook !== 'function')
      throw new TypeError('Expected callable guard buildEnd')
    const resolutions: string[] = []
    const parses = spyOn(wasm, 'compiledGuardFacts')
    guard.buildEnd = function (error) {
      return hook.call(
        new Proxy(this, {
          get(context, key) {
            if (key === 'resolve')
              return (...args: Parameters<typeof context.resolve>) => {
                resolutions.push(args[0])
                return context.resolve(...args)
              }
            return Reflect.get(context, key)
          },
        }),
        error,
      )
    }
    const provider: Plugin = {
      name: 'native-executable-provider',
      resolveId(id, importer) {
        if (id === 'virtual-executable') return '\0virtual-executable'
        if (id === './ordinary.js' && importer === '\0virtual-executable')
          return `${fixture.root}/src/ordinary.js`
      },
      load(id) {
        if (id === '\0virtual-executable') return code
      },
    }
    try {
      const expected = await fixture.run(false, [], { plugins: [provider] })
      // When native Vite builds with the delegating public guard observer.
      const actual = await fixture.run(false, [], {
        plugins: [provider, guard],
      })
      // Then output stays byte-identical without source parsing or late lookups.
      expect(actual).toEqual(expected)
      expect(parses.mock.calls.some(([source]) => source === code)).toBe(false)
      expect(resolutions).toEqual([])
    } finally {
      parses.mockRestore()
      await fixture.close()
    }
  },
)
