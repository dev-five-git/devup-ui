import * as fs from 'node:fs'
import { dirname, join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import type { Core } from '../coordinator-options'
import { type SourceWatcher, watchSources } from '../coordinator-watch'
import {
  createTestApp,
  eventually,
  failure,
  removeTestApps,
} from './coordinator-app'

afterEach(removeTestApps)

const source =
  'import { Box } from "@devup-ui/react"; export const A = <Box bg="red" />'

async function closeWatcher(watcher: SourceWatcher, core: Core): Promise<void> {
  watcher.close()
  await core.flush()
}

async function observed(probe: () => boolean, stage: string): Promise<void> {
  const deadline = Date.now() + 1000
  await eventually(() => {
    if (probe()) return true
    if (Date.now() > deadline)
      throw new Error(`Missing filesystem event: ${stage}`)
    return undefined
  })
}

describe('source root lifecycle without HTTP', () => {
  it.each(
    ['src', 'app', 'pages', 'nested/src/app'].flatMap((root) =>
      ['delete', 'rename'].map((action) => [root, action]),
    ),
  )(
    'reconciles extracted files after missing %s is created then %s',
    async (root, action) => {
      const app = createTestApp()
      const core = createCore(app.options({ watch: true }), app.root)
      await core.startup()
      let changes = 0
      const errors: Error[] = []
      const watcher = watchSources({
        roots: [join(app.root, root)],
        debounceMs: 10,
        onChange: () => {
          changes += 1
          void core.reconcile().catch((error: Error) => errors.push(error))
        },
        onError: (error) => errors.push(error),
      })
      try {
        const filename = app.write(`${root}/page.tsx`, source)
        await eventually(() => (changes > 0 ? true : undefined))
        await core.extract({ filename, resourcePath: filename, code: source })
        const query = { fileNum: 0, importMainCss: true, wait: false }
        expect((await core.css(query)).css).toContain('background:red')

        const before = changes
        if (action === 'delete') fs.rmSync(filename)
        else fs.renameSync(join(app.root, root), join(app.root, 'moved'))
        await eventually(async () => {
          const css = (await core.css(query)).css
          return changes > before && !css.includes('background:red')
            ? true
            : undefined
        })
        expect(errors).toEqual([])
      } finally {
        await closeWatcher(watcher, core)
      }
    },
  )

  it('removes fixtures after queued reconciliation when the test body fails', async () => {
    // Given a real core with durable output and a source watcher.
    const app = createTestApp()
    const runRoot = dirname(app.root)
    const core = createCore(
      app.options({
        watch: true,
        revisionFile: join(app.root, 'df', 'revision'),
      }),
      app.root,
    )
    await core.startup()
    const filename = app.write('src/page.tsx', source)
    await core.extract({ filename, resourcePath: filename, code: source })
    const watcher = watchSources({
      roots: [join(app.root, 'src')],
      debounceMs: 10,
      onChange: () => undefined,
      onError: (error) => {
        throw error
      },
    })
    const bodyError = new Error('fixture body failed')
    fs.rmSync(filename)
    const reconciliation = core.reconcile()
    try {
      // When failure reaches the same finally cleanup as the lifecycle tests.
      const error = await failure(
        (async () => {
          try {
            throw bodyError
          } finally {
            await closeWatcher(watcher, core)
            removeTestApps()
          }
        })(),
      )
      await reconciliation

      // Then the failure propagates and no queued write recreates the fixture.
      expect(error).toBe(bodyError)
      expect(fs.existsSync(runRoot)).toBe(false)
    } finally {
      watcher.close()
      await core.flush()
      fs.rmSync(runRoot, { recursive: true, force: true })
    }
  })

  it('reattaches after rapid replacements and observes deletion from the new root', async () => {
    const app = createTestApp()
    const root = join(app.root, 'src')
    fs.mkdirSync(root)
    let changes = 0
    const errors: Error[] = []
    const watcher = watchSources({
      roots: [root],
      debounceMs: 10,
      onChange: () => {
        changes += 1
      },
      onError: (error) => errors.push(error),
    })
    try {
      fs.renameSync(root, join(app.root, 'old'))
      fs.mkdirSync(root)
      fs.renameSync(root, join(app.root, 'also-old'))
      const filename = app.write('src/deep/new.tsx', source)
      await eventually(() => (changes > 0 ? true : undefined))
      const before = changes

      fs.rmSync(filename)
      await eventually(() => (changes > before ? true : undefined))

      expect(errors).toEqual([])
    } finally {
      watcher.close()
    }
  })

  it('moves ancestor hints as nested missing directories appear and are replaced', async () => {
    const app = createTestApp()
    let changes = 0
    const errors: Error[] = []
    const watcher = watchSources({
      roots: [join(app.root, 'nested', 'src', 'app')],
      debounceMs: 10,
      onChange: () => {
        changes += 1
      },
      onError: (error) => errors.push(error),
    })
    try {
      fs.mkdirSync(join(app.root, 'nested'))
      await observed(() => changes > 0, 'intermediate creation')
      const before = changes
      app.write('nested/src/app/page.tsx', source)
      await observed(() => changes > before, 'nested root creation')
      fs.rmSync(join(app.root, 'nested'), { recursive: true })
      app.write('nested/src/app/deep/new.tsx', source)
      const replaced = changes
      await observed(() => changes > replaced, 'ancestor replacement')

      const attached = changes
      fs.rmSync(join(app.root, 'nested', 'src', 'app'), { recursive: true })
      await observed(() => changes > attached, 'replacement root deletion')
      expect(errors).toEqual([])
    } finally {
      watcher.close()
    }
  })
})
