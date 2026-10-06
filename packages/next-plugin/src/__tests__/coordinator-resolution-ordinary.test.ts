import { afterEach, expect, it } from 'bun:test'

import { createCore } from '../coordinator-core'
import { createTestApp, removeTestApps } from './coordinator-app'

afterEach(removeTestApps)

it('replays ordinary static CSS when a previously missing preferred candidate appears', async () => {
  // Given
  const app = createTestApp()
  app.write(
    'tsconfig.json',
    JSON.stringify({
      compilerOptions: {
        baseUrl: '.',
        paths: { palette: ['outside/earlier', 'outside/red'] },
      },
    }),
  )
  app.write('outside/red.js', 'export const color = "red";')
  const code =
    'import {Box} from "@devup-ui/react"; import {color} from "palette"; export const Page=()=> <Box color={color}/>'
  const resourcePath = app.write('app/page.jsx', code)
  const core = createCore(
    app.options({ watch: true, singleCss: true, sourceRoots: [] }),
    app.root,
  )
  const request = { filename: 'app/page.jsx', resourcePath, code }
  try {
    await core.startup()
    await core.extract(request)
    expect(
      (
        await core.css({
          fileNum: undefined,
          importMainCss: false,
          wait: false,
        })
      ).css,
    ).toContain('color:red')
    // When
    app.write('outside/earlier.js', 'export const color = "blue";')
    await core.extract(request)
    const result = await core.css({
      fileNum: undefined,
      importMainCss: false,
      wait: false,
    })
    // Then
    expect(result.css).toContain('color:blue')
    expect(result.css).not.toContain('color:red')
  } finally {
    await core.flush()
    core.close()
  }
})

it('keeps the ordinary live engine and proof when replay watch adoption fails', async () => {
  // Given
  const app = createTestApp()
  app.write(
    'tsconfig.json',
    JSON.stringify({
      compilerOptions: {
        baseUrl: '.',
        paths: { palette: ['outside/earlier', 'outside/red'] },
      },
    }),
  )
  app.write('outside/red.js', 'export const color = "red";')
  const code =
    'import {Box} from "@devup-ui/react"; import {color} from "palette"; export const Page=()=> <Box color={color}/>'
  const resourcePath = app.write('app/page.jsx', code)
  const wasm = app.engine()
  const core = createCore(
    app.options({ wasm, watch: true, singleCss: true }),
    app.root,
  )
  try {
    await core.startup()
    await core.extract({ filename: 'app/page.jsx', resourcePath, code })
    if (!core.watchInputs || !core.onWatchInputs)
      throw new TypeError('Core has no watch input API')
    const previous = core.watchInputs()
    let refusals = 1
    const stop = core.onWatchInputs(() => {
      if (refusals > 0) {
        refusals -= 1
        throw new Error('watch adoption refused')
      }
    })
    app.write('outside/earlier.js', 'export const color = "blue";')
    // When
    await expect(core.reconcile()).rejects.toThrow('watch adoption refused')
    // Then
    expect(wasm.getCss(null, false)).toContain('color:red')
    expect(wasm.getCss(null, false)).not.toContain('color:blue')
    expect(core.watchInputs()).toEqual(previous)
    await core.reconcile()
    expect(
      (
        await core.css({
          fileNum: undefined,
          importMainCss: false,
          wait: false,
        })
      ).css,
    ).toContain('color:blue')
    stop()
  } finally {
    await core.flush()
    core.close()
  }
})
