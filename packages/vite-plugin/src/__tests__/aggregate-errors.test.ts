import { expect, it } from 'bun:test'

import { AggregateCssError } from '../aggregate-css'
import { fixture, module } from './aggregate-fixture'

const failures: readonly {
  readonly name: string
  readonly setup: (f: ReturnType<typeof fixture>) => void
  readonly file: string
  readonly reason: string
}[] = [
  {
    name: 'empty inputs',
    setup(f) {
      f.preparation.start(f.environment, undefined, { cssCodeSplit: false })
    },
    file: 'aggregate.css',
    reason: 'input list is empty',
  },
  {
    name: 'unresolved entry',
    setup(f) {
      f.context.resolve = async () => null
    },
    file: 'entry',
    reason: 'entry could not be resolved',
  },
  {
    name: 'external entry',
    setup(f) {
      f.context.resolve = async (id) => ({ id, external: 'absolute' })
    },
    file: 'entry',
    reason: 'entry could not be resolved',
  },
  {
    name: 'unresolved dependency',
    setup(f) {
      f.modules.set('entry', module("import 'missing'", ['missing']))
      f.context.resolve = async (id) =>
        id === 'entry' ? { id, external: false } : null
    },
    file: 'entry',
    reason: 'resolution refused missing',
  },
  {
    name: 'cyclic canonical resolution',
    setup(f) {
      f.modules.set('entry', module("import 'alias'", ['alias']))
      f.context.resolve = async (id) => ({
        id: id === 'alias' ? 'canonical' : id === 'canonical' ? 'alias' : id,
        external: false,
      })
    },
    file: 'entry',
    reason: 'ID cycle',
  },
  {
    name: 'null load',
    setup(f) {
      f.modules.delete('entry')
    },
    file: 'entry',
    reason: 'null or incomplete',
  },
  {
    name: 'incomplete code',
    setup(f) {
      f.modules.set('entry', { ...module(), code: null })
    },
    file: 'entry',
    reason: 'null or incomplete',
  },
  {
    name: 'incomplete static edges',
    setup(f) {
      Object.defineProperty(f.modules.get('entry'), 'importedIds', {
        value: null,
      })
    },
    file: 'entry',
    reason: 'null or incomplete',
  },
  {
    name: 'incomplete dynamic edges',
    setup(f) {
      Object.defineProperty(f.modules.get('entry'), 'dynamicallyImportedIds', {
        value: null,
      })
    },
    file: 'entry',
    reason: 'null or incomplete',
  },
  {
    name: 'unresolved import graph',
    setup(f) {
      f.modules.set('entry', module("export * from 'missing'"))
    },
    file: 'entry',
    reason: 'unresolved import graph',
  },
  {
    name: 'computed import',
    setup(f) {
      f.modules.set(
        'entry',
        module("const target='late';\nexport const lazy=()=>import(target)"),
      )
    },
    file: 'entry',
    reason: 'computed dynamic import',
  },
  {
    name: 'computed template import',
    setup(f) {
      f.modules.set(
        'entry',
        module('export const lazy=(name)=>import(`./${name}.js`)'),
      )
    },
    file: 'entry',
    reason: 'computed dynamic import',
  },
  {
    name: 'missing import source',
    setup(f) {
      f.context.parse = () => ({
        type: 'Program',
        body: [{ type: 'ImportExpression' }],
      })
    },
    file: 'entry',
    reason: 'incomplete dynamic import',
  },
  {
    name: 'unknown dynamic source',
    setup(f) {
      f.context.parse = () => ({
        type: 'Program',
        body: [{ type: 'ImportExpression', source: { type: 'Identifier' } }],
      })
    },
    file: 'entry',
    reason: 'computed dynamic import',
  },
  {
    name: 'null parser',
    setup(f) {
      f.context.parse = () => null
    },
    file: 'entry',
    reason: 'incomplete module information',
  },
  {
    name: 'incomplete parser',
    setup(f) {
      f.context.parse = () => ({})
    },
    file: 'entry',
    reason: 'incomplete module information',
  },
  {
    name: 'invalid parser root',
    setup(f) {
      f.context.parse = () => ({ type: 'Literal' })
    },
    file: 'entry',
    reason: 'incomplete module information',
  },
  ...[new Error('refusal'), 'non-error refusal'].flatMap((cause) => [
    {
      name: `load rejection ${String(cause)}`,
      setup(f: ReturnType<typeof fixture>) {
        f.context.load = async () => {
          throw cause
        }
      },
      file: 'entry',
      reason: 'public load refused',
    },
    {
      name: `resolve rejection ${String(cause)}`,
      setup(f: ReturnType<typeof fixture>) {
        f.context.resolve = async () => {
          throw cause
        }
      },
      file: 'entry',
      reason: 'public resolution refused',
    },
    {
      name: `parse rejection ${String(cause)}`,
      setup(f: ReturnType<typeof fixture>) {
        f.context.parse = () => {
          throw cause
        }
      },
      file: 'entry',
      reason: 'public parser refused',
    },
  ]),
]

for (const failure of failures) {
  it(`fails closed with a located asset/config alternative when ${failure.name}`, async () => {
    // Given: a public seam refusing a complete transformed closure.
    const f = fixture()
    failure.setup(f)
    // When: a generated snapshot attempts preparation.
    try {
      await f.preparation.prepare(f.environment, f.context, 'aggregate.css')
      throw new Error('Expected aggregate preparation failure')
    } catch (error) {
      // Then: no silent partial success; responsible file and alternatives survive.
      expect(error).toBeInstanceOf(AggregateCssError)
      if (!(error instanceof AggregateCssError)) throw error
      expect(error.file).toBe(failure.file)
      expect(error.asset).toBe('aggregate.css')
      expect(error.message).toContain(failure.reason)
      expect(error.message).toContain('build.cssCodeSplit=true')
      expect(error.message).toContain('separate imported asset')
      if (failure.name === 'computed import') {
        expect(error.line).toBe(2)
        expect(error.column).toBe(23)
      }
    }
  })
}

it('locates an unsafe standalone asset even when a plugin supplied all entry roots', () => {
  const f = fixture()
  f.preparation.start(f.environment, undefined, { cssCodeSplit: false })
  f.preparation.checkAsset(f.environment, { fileName: 'image.svg' })
  expect(() =>
    f.preparation.checkAsset(f.environment, {
      name: 'devup-ui.css',
      fileName: 'custom/devup-ui-hash.css',
    }),
  ).toThrow('custom/devup-ui-hash.css:1:1: [devup-ui] aggregate CSS asset')
})
