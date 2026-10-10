import { readFileSync } from 'node:fs'
import * as fsp from 'node:fs/promises'
import { join } from 'node:path'

import { afterEach, expect, it, spyOn } from 'bun:test'

import { createCore } from '../coordinator-core'
import {
  type CoordinatorInstance,
  createInstance,
} from '../coordinator-instance'
import { createInput } from '../coordinator-ledger'
import { readCoordinatorState } from '../state'
import { connect, createTestApp, removeTestApps } from './coordinator-app'
import {
  box,
  compiledSource,
  cssQuery,
  generation,
  preparedCore,
  prewarmed,
} from './coordinator-prepared-fixture'

const instances: CoordinatorInstance[] = []
afterEach(() => {
  for (const instance of instances.splice(0)) instance.close()
  removeTestApps()
})

it.each([false, true])(
  'adopts fresh complete prepared inputs (empty=%s) instead of unchanged backing checkpoint',
  async (empty) => {
    // Given a durable compiled checkpoint whose raw Markdown is unchanged.
    const app = createTestApp()
    app.write('src/page.mdx', '# unchanged Markdown')
    const first = await preparedCore(app, {
      generation: generation([
        compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
      ]),
      revision: 3,
    })
    await first.core.flush()
    first.core.close()
    const checkpoint = readCoordinatorState(first.options.stateFile ?? '', '')
    if (checkpoint === undefined) throw new Error('fixture checkpoint missing')
    const next = prewarmed(app, {
      checkpoint,
      revision: 4,
      generation: generation(
        empty
          ? []
          : [
              compiledSource(app, {
                filename: 'src/page.mdx',
                code: box('green'),
              }),
            ],
      ),
    })
    let writes = 0
    const rename = fsp.rename
    const observed = spyOn(fsp, 'rename').mockImplementation(
      async (from, to) => {
        if (to === next.options.stateFile) writes += 1
        return rename(from, to)
      },
    )
    try {
      // When a real HTTP coordinator starts from the complete prepared engine.
      const instance = createInstance(next.options)
      instances.push(instance)
      await instance.ready
      const reply = await connect(app.portFile, app.identity).get(
        '/css?fileNum=0',
      )

      // Then the first stylesheet/checkpoint are fresh, with one startup write and no replay.
      expect(reply.status).toBe(200)
      expect(reply.body).not.toContain('background:red')
      if (!empty) expect(reply.body).toContain('background:green')
      const current = readCoordinatorState(next.options.stateFile ?? '', '')
      expect(current?.revision).toBe(4)
      expect(current?.inputs.map((input) => input.source)).toEqual(
        empty ? [] : [box('green')],
      )
      expect(current?.fileMap).toEqual(checkpoint.fileMap)
      expect(writes).toBe(1)
      expect(next.extractions).toEqual(empty ? [] : ['map:src/page.mdx'])
      const later = await connect(app.portFile, app.identity).post(
        '/extract',
        app.post('src/z.tsx', box('blue')),
      )
      expect(JSON.parse(later.body).cssFile).toEndWith('devup-ui-1.css')
      await instance.drain()
    } finally {
      observed.mockRestore()
    }
  },
)

it('preserves sorted production prewarm and seals without invoking development preparation', async () => {
  // Given a complete prepared generation in reverse input order.
  const app = createTestApp()
  const sources = ['z', 'a'].map((name) => {
    app.write(`src/${name}.mdx`, '# raw Markdown')
    return compiledSource(app, {
      filename: `src/${name}.mdx`,
      code: box(name === 'a' ? 'red' : 'blue'),
    })
  })
  const prepared = prewarmed(app, {
    generation: generation(sources),
    prepareReplay: async () => {
      throw new Error('development provider must not run')
    },
  })
  const core = createCore({ ...prepared.options, watch: false }, app.root)
  await core.startup()

  // When production requests its complete stylesheet and re-extracts the same prepared bytes.
  await core.css({ ...cssQuery, fileNum: undefined })
  const output = await core.extract({
    filename: 'src/a.mdx',
    resourcePath: join(app.root, 'src/a.mdx'),
    code: box('red'),
  })

  // Then the initial path ordering/production cache remains unchanged.
  expect(prepared.extractions).toEqual(['map:src/a.mdx', 'map:src/z.mdx'])
  expect(output.cssFile).toEndWith('devup-ui-0.css')
  await core.flush()
  core.close()
})

it('does not trust persisted compiled Markdown in standalone checkpoint replay', async () => {
  // Given a checkpoint containing ordinary JS and compiled .md/.mdx with unchanged raw backing.
  const app = createTestApp()
  const sources = ['a.md', 'b.mdx'].map((filename) => {
    app.write(filename, '# raw Markdown')
    return compiledSource(app, { filename, code: box('red') })
  })
  app.write('ordinary.tsx', box('blue'))
  const ordinary = createInput(
    app.root,
    {
      filename: 'ordinary.tsx',
      resourcePath: join(app.root, 'ordinary.tsx'),
      code: box('blue'),
    },
    [],
  )
  const first = await preparedCore(app, {
    generation: generation(sources),
    ordinaryInputs: [ordinary],
  })
  await first.core.flush()
  first.core.close()
  const standalone = createCore(
    { ...first.options, wasm: app.engine(), preparedSources: undefined },
    app.root,
  )

  // When the old standalone checkpoint path starts without fresh compiled proof.
  await standalone.startup()

  // Then ordinary inputs still replay, while neither Markdown source is authorized by its raw stamp.
  expect(
    readCoordinatorState(first.options.stateFile ?? '', '')?.inputs.map(
      (input) => input.filename,
    ),
  ).toEqual(['ordinary.tsx'])
  expect((await standalone.css({ ...cssQuery, fileNum: 2 })).css).toContain(
    'background:blue',
  )
  standalone.close()
})

it('keeps provider preparation-time stamps instead of restamping prewarmed compiled bytes', async () => {
  // Given a prewarmed source captured before a compiler dependency changed.
  const app = createTestApp()
  app.write('src/page.mdx', '# raw')
  const dep = app.write('compiler-input.txt', 'old')
  const source = compiledSource(app, {
    filename: 'src/page.mdx',
    code: box('red'),
    dependencies: [dep],
  })
  const prepared = prewarmed(app, { generation: generation([source]) })
  app.write('compiler-input.txt', 'new')

  // When Core adopts and commits the supplied complete generation.
  const core = createCore(prepared.options, app.root)
  await core.startup()

  // Then the durable source retains the old preparation evidence, not a false fresh stamp.
  const snapshot = readCoordinatorState(prepared.options.stateFile ?? '', '')
  expect(snapshot?.inputs[0]?.stamps).toEqual(source.input.stamps)
  expect(readFileSync(dep, 'utf8')).toBe('new')
  core.close()
})
