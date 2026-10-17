import { exportFileMap } from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { NonphysicalModules } from '../nonphysical-modules'
import { productionFixture } from './production-fixture'

function numbers(): Readonly<Record<string, number>> {
  const value: unknown = JSON.parse(exportFileMap())
  if (value === null || typeof value !== 'object' || Array.isArray(value))
    throw new TypeError('Expected native file numbering object')
  return Object.fromEntries(
    Object.entries(value).map(([id, ordinal]) => {
      if (typeof ordinal !== 'number')
        throw new TypeError('Expected numeric file ordinal')
      return [id, ordinal]
    }),
  )
}

it.each([false, true])(
  'preseeds physical roots before opposite native bridge arrivals with singleCss=%s',
  async (singleCss) => {
    // Given independent cold stores and styles visible only through a virtual bridge.
    const results = []
    for (const reverse of [false, true]) {
      const fixture = await productionFixture()
      const a = await fixture.file(
        'app/src/a-style.js',
        "import {css} from '@devup-ui/react'; export const a=css({zIndex:11});",
      )
      const z = await fixture.file(
        'app/src/z-style.js',
        "import {css} from '@devup-ui/react'; export const z=css({zIndex:22});",
      )
      const unused = await fixture.file(
        'app/src/m-unused.js',
        "import {css} from '@devup-ui/react'; export const unused=css({zIndex:99});",
      )
      await fixture.file(
        'app/src/main.js',
        "export {a,z} from 'virtual:root-bridge';",
      )
      const expected = [a, unused, z].sort()
      const order = reverse ? [z, a] : [a, z]
      const arrivals: string[] = []
      const seeds: Readonly<Record<string, number>>[] = []
      const original = NonphysicalModules.prototype.extract
      const extract = spyOn(
        NonphysicalModules.prototype,
        'extract',
      ).mockImplementation(function (
        this: NonphysicalModules,
        root,
        resolver,
        action,
      ) {
        seeds.push(numbers())
        if (order.includes(root.id)) arrivals.push(root.id)
        return original.call(this, root, resolver, action)
      })
      try {
        // When public sequential loads force opposite real extraction orders.
        const output = await fixture.run(singleCss, [
          {
            name: 'physical-root-bridge',
            resolveId(id) {
              if (id === 'virtual:root-bridge') return '\0root-bridge.js'
            },
            async load(id) {
              if (id !== '\0root-bridge.js') return
              for (const path of order) await this.load({ id: path })
              return `export {a} from ${JSON.stringify(a)}; export {z} from ${JSON.stringify(z)};`
            },
          },
        ])
        const css = output
          .filter((item) => item.type === 'asset')
          .map((item) => item.source)
          .join('')
        const js = output
          .filter((item) => item.type === 'chunk')
          .map((item) => item.code)
          .join('')
        const classes = [11, 22].map((value) => {
          const rule = new RegExp(
            `\\.([\\w-]+)\\s*\\{[^}]*z-index\\s*:\\s*${value}(?=[;}\\s])`,
          ).exec(css)
          expect(rule?.[1]).toBeDefined()
          expect(js).toContain(rule?.[1] ?? 'missing-class')
          return rule?.[1]
        })
        const map = numbers()
        const learned = await fixture.learned()
        console.info(
          JSON.stringify({
            singleCss,
            reverse,
            order,
            arrivals,
            seeds,
            map,
            learned,
            classes,
          }),
        )
        // Then arrival reverses while root ordinals and emitted styling stay correct.
        expect(arrivals).toEqual(order)
        expect(css).not.toContain('z-index:99')
        expect(js).not.toContain('@devup-ui/react')
        results.push({ expected, seeds, map, learned, classes })
      } finally {
        extract.mockRestore()
        await fixture.close()
      }
    }
    for (const result of results) {
      const expected = Object.fromEntries(
        result.expected.map((id, ordinal) => [id, ordinal]),
      )
      expect(result.seeds[0]).toEqual(expected)
      expect(result.map).toEqual(expected)
      for (const id of result.expected) expect(result.learned).not.toContain(id)
    }
    // Single-CSS atom-order independence belongs to PR B's shared Vite slots.
    if (!singleCss) expect(results[0]?.classes).toEqual(results[1]?.classes)
  },
  60000,
)
