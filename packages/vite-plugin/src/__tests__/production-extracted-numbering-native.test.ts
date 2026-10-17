import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import { NonphysicalModules } from '../nonphysical-modules'
import { DevupUI } from '../plugin'
import { productionFixture } from './production-fixture'

function fileIds(): string[] {
  const map: unknown = JSON.parse(wasm.exportFileMap())
  if (map === null || typeof map !== 'object' || Array.isArray(map))
    throw new TypeError('Expected real WASM file map')
  return Object.keys(map).sort()
}

it.each([false, true])(
  'reserves root candidates and reached dist and test modules with singleCss=%s',
  async (singleCss) => {
    // Given an independently specified cohort and four unreachable counterexamples.
    const fixture = await productionFixture()
    const main = await fixture.file(
      'app/src/main.js',
      "import {css} from '@devup-ui/react'; import {rank} from './tokens.js'; export {dist} from '@devup-ui/fixture'; export {test} from './reached.test.js'; export {page} from './page.js'; export const main=css({zIndex:rank});",
    )
    const page = await fixture.file(
      'app/src/page.js',
      "import {styled} from '@devup-ui/react'; export const page=styled('h1',{zIndex:44});",
    )
    const tokensCode = "/* @devup-ui/react */ export {rank} from './rank.js';"
    const rankCode = '/* @devup-ui/react */ export const rank=11;'
    const tokens = await fixture.file('app/src/tokens.js', tokensCode)
    const rank = await fixture.file('app/src/rank.js', rankCode)
    const reached = await fixture.file(
      'app/src/reached.test.js',
      "import {css} from '@devup-ui/react'; export const test=css({zIndex:22});",
    )
    await fixture.file(
      'app/node_modules/@devup-ui/fixture/package.json',
      '{"name":"@devup-ui/fixture","type":"module","exports":"./dist/view.js"}',
    )
    const dist = await fixture.file(
      'app/node_modules/@devup-ui/fixture/dist/view.js',
      "import {css} from '@devup-ui/react'; export const dist=css({zIndex:33});",
    )
    await fixture.file(
      'app/node_modules/@devup-ui/tooling/package.json',
      '{"name":"@devup-ui/tooling","type":"module","main":"index.js"}',
    )
    const unused = await fixture.file(
      'app/src/aaa-unused.js',
      "import {css} from '@devup-ui/react'; import 'foreign-poison'; import '@devup-editor/root-only'; export const unused=css({zIndex:99});",
    )
    await fixture.file(
      'app/node_modules/foreign-poison/package.json',
      '{broken',
    )
    await fixture.file(
      'app/node_modules/@devup-editor/root-only/package.json',
      '{"name":"@devup-editor/root-only","exports":"./dist/style.js"}',
    )
    const rootOnly = await fixture.file(
      'app/node_modules/@devup-editor/root-only/dist/style.js',
      "import {css} from '@devup-ui/react'; export const rootOnly=css({zIndex:88});",
    )
    const negatives = []
    for (const path of [
      'app/src/fixtures/unused.js',
      'app/src/tooling/unused.js',
      'app/src/dist/unused.js',
      'app/node_modules/@devup-ui/tooling/index.js',
      'app/node_modules/@devup-ui/fixture/dist/unused.js',
    ])
      negatives.push(
        await fixture.file(
          path,
          "import {css} from '@devup-ui/react'; export const unused=css({zIndex:99});",
        ),
      )
    negatives.push(
      await fixture.file(
        'app/src/aaa-fixture.test.js',
        'export const fixture="import {css} from \'@devup-ui/react\'";',
      ),
    )
    const expected = [main, tokens, rank, reached, dist, page].sort()
    const reservedExpected = [...expected, unused, rootOnly].sort()
    const seeds: string[][] = []
    const completed: string[][] = []
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
      seeds.push(fileIds())
      return original.call(this, root, resolver, action)
    })
    const direct = spyOn(NonphysicalModules.prototype, 'direct')
    const recursive = spyOn(NonphysicalModules.prototype, 'resolved')
    try {
      // When the real Vite pipeline executes unchanged production and real WASM.
      const output = await fixture.run(singleCss, [], {
        build: {
          write: false,
          lib: { entry: main, formats: ['es'] },
          rollupOptions: { external: ['react/jsx-runtime'] },
        },
        plugins: [
          DevupUI({
            singleCss,
            include: ['@devup-ui/fixture'],
            sourceDirs: ['src'],
          }),
          {
            name: 'observe-numbering-red',
            generateBundle() {
              completed.push(fileIds())
            },
          },
        ],
      })
      // Then styling and evaluator recursion work before reservation is checked.
      const css = output
        .filter((item) => item.type === 'asset')
        .map((item) => item.source)
        .join('')
      const js = output
        .filter((item) => item.type === 'chunk')
        .map((item) => item.code)
        .join('')
      for (const value of [11, 22, 33, 44]) {
        const rule = new RegExp(
          `\\.([\\w-]+)\\s*\\{[^}]*z-index\\s*:\\s*${value}(?=[;}\\s])`,
        ).exec(css)
        expect(rule?.[1]).toBeDefined()
        expect(js).toContain(rule?.[1] ?? 'missing-class')
      }
      expect(css).not.toContain('z-index:99')
      expect(css).not.toContain('z-index:88')
      expect(js).not.toContain('@devup-ui/react')
      const roots = [...new Set(direct.mock.calls.map(([, id]) => id))].sort()
      const returns = recursive.mock.calls.flatMap(([, result]) => {
        if (result === undefined) return []
        switch (result.ignored) {
          case true:
            return []
          case undefined:
            return [{ id: result.path, code: result.code }]
          default:
            return result satisfies never
        }
      })
      expect(roots).toContain(main)
      expect(roots).toContain(dist)
      expect(roots).toContain(reached)
      expect(returns).toContainEqual({ id: tokens, code: tokensCode })
      expect(returns).toContainEqual({ id: rank, code: rankCode })
      const observed = [
        ...new Set([...roots, ...returns.map(({ id }) => id)]),
      ].sort()
      for (const id of expected) expect(observed).toContain(id)
      for (const id of negatives) expect(observed).not.toContain(id)
      expect(seeds.length).toBeGreaterThan(0)
      expect(completed).toHaveLength(1)
      const reserved = completed[0] ?? []
      console.info(
        JSON.stringify({
          case: 'numbering-red',
          singleCss,
          expected,
          direct: roots,
          recursive: returns,
          seeds,
          reserved,
          extra: reserved.filter((id) => !expected.includes(id)),
          missing: expected.filter((id) => !reserved.includes(id)),
          stylingAndRecursion: 'passed',
        }),
      )
      expect(reserved).toEqual(reservedExpected)
      expect(seeds[0]).toEqual(reservedExpected)
      const numbers: unknown = JSON.parse(wasm.exportFileMap())
      if (numbers === null || typeof numbers !== 'object')
        throw new TypeError('Expected real numbering object')
      expect(Object.getOwnPropertyDescriptor(numbers, page)?.value).toBe(
        reservedExpected.filter((id) => id < page).length,
      )
      for (const id of negatives)
        expect(Object.getOwnPropertyDescriptor(numbers, id)).toBeUndefined()
    } finally {
      recursive.mockRestore()
      direct.mockRestore()
      extract.mockRestore()
      await fixture.close()
    }
  },
  60000,
)
