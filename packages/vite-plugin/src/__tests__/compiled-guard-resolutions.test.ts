import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'
import type { Plugin } from 'vite'

import { createCompiledGuard } from '../compiled-guard'
import { productionFixture } from './production-fixture'

it.each([false, true])(
  'omits unused guard resolutions with identical native output when virtual=%s',
  async (virtual) => {
    // Given executable imports that are not forwarding export origins.
    const fixture = await productionFixture()
    const code =
      'import {read} from "./ordinary.js"; export const result=read()'
    const entry = await fixture.file(
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
    const hook = guard.moduleParsed
    if (typeof hook !== 'function')
      throw new TypeError('Expected callable guard moduleParsed')
    const resolutions: string[] = []
    const parses = spyOn(wasm, 'compiledGuardFacts')
    guard.moduleParsed = function (info) {
      return hook.call(
        new Proxy(this, {
          get(context, key) {
            if (key === 'resolve')
              return (...args: Parameters<typeof context.resolve>) => {
                if (info.id === (virtual ? '\0virtual-executable' : entry))
                  resolutions.push(args[0])
                return context.resolve(...args)
              }
            return Reflect.get(context, key)
          },
        }),
        info,
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
      // Then output stays byte-identical and parsing stays active, without lookups.
      expect(actual).toEqual(expected)
      expect(parses.mock.calls.some(([source]) => source === code)).toBe(true)
      expect(resolutions).toEqual([])
    } finally {
      parses.mockRestore()
      await fixture.close()
    }
  },
)
