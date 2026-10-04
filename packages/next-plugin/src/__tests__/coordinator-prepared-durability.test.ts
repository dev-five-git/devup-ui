import { readFileSync } from 'node:fs'
import * as fsp from 'node:fs/promises'

import { afterEach, expect, it, spyOn } from 'bun:test'

import {
  type CoordinatorInstance,
  createInstance,
} from '../coordinator-instance'
import type { PreparedSourceGeneration } from '../coordinator-options'
import { readCoordinatorState } from '../state'
import {
  connect,
  createTestApp,
  failure,
  removeTestApps,
} from './coordinator-app'
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

it.each(['stateFile', 'revisionFile'] as const)(
  'keeps prior durable/live state on %s candidate failure and retries with one revision',
  async (destination) => {
    // Given a durable prepared app and an unrelated live HTTP app.
    const app = createTestApp()
    app.write('src/page.mdx', '# old')
    const seen: string[] = []
    const staged: { generation?: PreparedSourceGeneration } = {}
    const prepared = prewarmed(app, {
      generation: generation([
        compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
      ]),
      prepareReplay: async ({ generation: current }) => {
        seen.push(current.sources[0]?.input.source ?? '')
        return current.sources[0]?.input.source === box('green')
          ? current
          : (staged.generation ?? current)
      },
    })
    const instance = createInstance(prepared.options)
    instances.push(instance)
    await instance.ready
    const client = connect(app.portFile, app.identity)
    const other = createTestApp()
    const otherInstance = createInstance(other.options({ watch: true }))
    instances.push(otherInstance)
    await otherInstance.ready
    const otherClient = connect(other.portFile, other.identity)
    await otherClient.post('/extract', other.post('ordinary.tsx', box('blue')))
    const stateFile = prepared.options.stateFile ?? ''
    const revisionFile = prepared.options.revisionFile ?? ''
    const checkpoint = readFileSync(stateFile, 'utf8')
    const revision = readFileSync(revisionFile, 'utf8')
    const target = prepared.options[destination]
    const rename = fsp.rename
    let blocked = true
    const observed = spyOn(fsp, 'rename').mockImplementation(
      async (from, to) => {
        if (to === target && blocked)
          throw new Error('controlled candidate durability failure')
        return rename(from, to)
      },
    )
    app.write('src/page.mdx', '# new')
    staged.generation = generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
    ])
    try {
      // When required replay encounters repeatable disk/revision publication failures.
      const failed = await client.get('/css?fileNum=0')
      const failedAgain = await client.get('/css?fileNum=0')

      // Then neither late CSS success nor partial live publication can escape the failure.
      expect(failed.status).toBe(500)
      expect(failedAgain.status).toBe(500)
      expect(readFileSync(stateFile, 'utf8')).toBe(checkpoint)
      expect(readFileSync(revisionFile, 'utf8')).toBe(revision)
      expect(prepared.wasm.getCss(0, false)).toContain('background:red')
      expect(await failure(instance.flush())).toBeInstanceOf(Error)
      expect((await otherClient.get('/css?fileNum=0')).body).toContain(
        'background:blue',
      )
      blocked = false
      const recovered = await client.get('/css?fileNum=0')
      expect(recovered.status).toBe(200)
      expect(recovered.body).toContain('background:green')
      expect(readCoordinatorState(stateFile, '')?.revision).toBe(8)
      expect(readFileSync(revisionFile, 'utf8')).toBe('8')
      expect(seen).toEqual([box('red'), box('red'), box('red')])
      await instance.drain()
      await otherInstance.drain()
    } finally {
      observed.mockRestore()
    }
  },
)

it.each(['css', 'extract', 'watch'] as const)(
  'blocks %s on required preparation failure while durable state stays recoverable',
  async (operation) => {
    // Given a provider retaining the original located compiler failure.
    const app = createTestApp()
    app.write('src/page.mdx', '# old')
    const cause = new Error(
      'src/page.mdx:4:7: controlled required preparation failure',
    )
    let blocked = true
    const staged: { generation?: PreparedSourceGeneration } = {}
    const prepared = await preparedCore(app, {
      generation: generation([
        compiledSource(app, { filename: 'src/page.mdx', code: box('red') }),
      ]),
      prepareReplay: async ({ generation: current }) => {
        if (blocked) throw cause
        return current.sources[0]?.input.source === box('green')
          ? current
          : (staged.generation ?? current)
      },
    })
    const before = readFileSync(prepared.options.stateFile ?? '', 'utf8')
    app.write('src/page.mdx', '# new')
    staged.generation = generation([
      compiledSource(app, { filename: 'src/page.mdx', code: box('green') }),
    ])

    // When a queued CSS/extract/watch operation needs that failing refresh.
    const pending =
      operation === 'css'
        ? prepared.core.css(cssQuery)
        : operation === 'watch'
          ? prepared.core.reconcile()
          : prepared.core.extract({
              filename: 'ordinary.tsx',
              resourcePath: `${app.root}/ordinary.tsx`,
              code: box('purple'),
            })

    // Then the original failure blocks work without destroying the durable predecessor; correction recovers.
    expect(await failure(pending)).toBe(cause)
    expect(readFileSync(prepared.options.stateFile ?? '', 'utf8')).toBe(before)
    expect(prepared.wasm.getCss(0, false)).toContain('background:red')
    blocked = false
    expect((await prepared.core.css(cssQuery)).css).toContain(
      'background:green',
    )
    expect(
      readCoordinatorState(prepared.options.stateFile ?? '', '')?.revision,
    ).toBe(8)
    prepared.core.close()
  },
)
