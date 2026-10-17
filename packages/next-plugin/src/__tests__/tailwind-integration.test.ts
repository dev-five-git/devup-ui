import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { resetCoordinator, startCoordinator } from '../coordinator'
import { DevupUI, reloadTurboSetupModuleForTesting } from '../plugin'
import { connect } from './coordinator-app'
import { box, installProjectHooks, makeProject } from './project'
import { installTurboHarness } from './turbo-harness'

installProjectHooks()
const harness = installTurboHarness()
afterEach(resetCoordinator)

const source = box('className="bg-brand"')

function project() {
  const root = makeProject({
    'src/app/page.tsx': source,
    'config/devup.json': JSON.stringify({ extends: ['./base.json'] }),
    'config/base.json': JSON.stringify({
      theme: { colors: { default: { primary: '#abcdef' } } },
      tailwind: { css: 'styles/tailwind.css' },
    }),
    'styles/tailwind.css': '@import "./brand.css";',
    'styles/brand.css': '@theme { --color-brand: #112233; }',
  })
  process.chdir(root)
  return root
}

function latestStart() {
  const start = harness.starts.at(-1)
  if (!start) throw new Error('Expected a captured coordinator')
  return start
}

describe('Next Tailwind theme integration', () => {
  it('registers inherited root-relative CSS before prewarm and tracks local imports in both loaders', () => {
    // Given a nested config and Tailwind files outside the source roots.
    const root = project()

    // When the real setup configures and prewarms its engine.
    const result = DevupUI(
      {},
      { devupFile: 'config/devup.json', singleCss: true },
    )
    const start = latestStart()

    // Then Tailwind utilities and the Devup theme coexist in emitted CSS.
    const css = start.wasm.getCss(undefined, false)
    expect(css).toMatch(/background(?:-color)?:#(?:112233|123)\b/)
    expect(css.toLowerCase()).toContain('--primary:#abcdef')
    const themeFiles = [
      'config/devup.json',
      'config/base.json',
      'styles/tailwind.css',
      'styles/brand.css',
    ].map((file) => join(root, file))
    const rules = Object.values(result.turbopack?.rules ?? {})
    expect(rules).toEqual(
      expect.arrayContaining([
        [
          expect.objectContaining({
            options: expect.objectContaining({ themeFiles }),
          }),
        ],
        expect.objectContaining({
          loaders: [
            expect.objectContaining({
              options: expect.objectContaining({ themeFiles }),
            }),
          ],
        }),
      ]),
    )
  })

  it('discovers a conventional Tailwind entry when devup.json has no Tailwind option', () => {
    // Given a conventional entry importing Tailwind.
    const root = makeProject({
      'src/app/page.tsx': source,
      'src/app/globals.css':
        '@import "tailwindcss"; @theme { --color-brand: #112233; }',
    })
    process.chdir(root)

    // When setup runs without a config file.
    const result = DevupUI({}, { singleCss: true })

    // Then its utility is extracted and its entry is a loader dependency.
    expect(latestStart().wasm.getCss(undefined, false)).toMatch(
      /background(?:-color)?:#(?:112233|123)\b/,
    )
    expect(JSON.stringify(result.turbopack?.rules)).toContain(
      JSON.stringify(join(root, 'src/app/globals.css')).slice(1, -1),
    )
  })

  it('honors disabled Tailwind discovery without losing the configured Devup theme', () => {
    // Given an explicitly disabled conventional Tailwind entry.
    const root = makeProject({
      'src/app/page.tsx': box('bg="$primary"'),
      'devup.json': JSON.stringify({
        tailwind: { css: false },
        theme: { colors: { default: { primary: '#abcdef' } } },
      }),
      'src/app/globals.css':
        '@import "tailwindcss"; @theme { --color-brand: #112233; }',
    })
    process.chdir(root)

    // When setup resolves its theme.
    const result = DevupUI({}, { singleCss: true })

    // Then only Devup's configured theme is registered.
    expect(latestStart().wasm.getCss(undefined, false).toLowerCase()).toContain(
      '--primary:#abcdef',
    )
    expect(JSON.stringify(result.turbopack?.rules)).not.toContain('tailwindCss')
    expect(JSON.stringify(result.turbopack?.rules)).not.toContain('globals.css')
  })

  it('invalidates the production handoff when an imported Tailwind file changes', () => {
    // Given an existing production setup whose config bytes do not change.
    const root = project()
    DevupUI({}, { devupFile: 'config/devup.json', singleCss: true })
    reloadTurboSetupModuleForTesting()
    writeFileSync(
      join(root, 'styles/brand.css'),
      '@theme { --color-brand: #445566; }',
    )

    // When the next config evaluation tries to consume that handoff.
    DevupUI({}, { devupFile: 'config/devup.json', singleCss: true })

    // Then a fresh setup prewarms the new utility rather than reusing stale CSS.
    expect(harness.starts).toHaveLength(2)
    expect(latestStart().wasm.getCss(undefined, false)).toMatch(
      /background(?:-color)?:#(?:445566|456)\b/,
    )
  })

  it('replays Tailwind-only edits and returns newly consumed dependencies against the captured app root', async () => {
    // Given a real development coordinator whose working directory later changes.
    const root = project()
    process.env.NODE_ENV = 'development'
    DevupUI({}, { devupFile: 'config/devup.json', singleCss: true })
    const start = latestStart()
    if (!start.identity) throw new Error('Expected coordinator ownership')
    harness.startRealCoordinators()
    const coordinator = startCoordinator(start)
    try {
      await coordinator.ready
      const client = connect(start.coordinatorPortFile, start.identity)
      process.chdir(makeProject())
      writeFileSync(
        join(root, 'styles/next.css'),
        '@theme { --color-brand: #445566; }',
      )
      writeFileSync(join(root, 'styles/tailwind.css'), '@import "./next.css";')

      // When an unchanged source requests extraction after the Tailwind-only edit.
      const reply = await client.post(
        '/extract',
        JSON.stringify({
          filename: 'src/app/page.tsx',
          resourcePath: join(root, 'src/app/page.tsx'),
          code: source,
        }),
      )
      const css = await client.get('/css')

      // Then replay refreshes real CSS and exposes the newly consumed dependency.
      expect(reply.status).toBe(200)
      expect(JSON.parse(reply.body).dependencies).toContain(
        join(root, 'styles/next.css'),
      )
      expect(css.status).toBe(200)
      expect(css.body).toMatch(/background(?:-color)?:#(?:445566|456)\b/)
      expect(css.body).not.toMatch(/background(?:-color)?:#(?:112233|123)\b/)
    } finally {
      await coordinator.drain()
    }
  })
})
