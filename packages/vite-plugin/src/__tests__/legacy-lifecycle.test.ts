import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { runBuildOperation } from '@devup-ui/plugin-utils/internal/build-admission'
import * as wasm from '@devup-ui/wasm'
import { afterEach, expect, it, spyOn } from 'bun:test'
import { type InlineConfig, type Plugin, resolveConfig } from 'vite'

import { DevupUI } from '../plugin'

const plugins = new Set<Plugin>()
const roots = new Set<string>()
afterEach(() => {
  for (const plugin of plugins) invoke(plugin, 'closeWatcher')
  plugins.clear()
  for (const root of roots) rmSync(root, { recursive: true, force: true })
  roots.clear()
})
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-lifecycle-'))
  roots.add(root)
  return root
}
type Options = Parameters<typeof DevupUI>[0]
type Hook = 'config' | 'configResolved' | 'closeBundle' | 'closeWatcher'
async function main(options: Options = {}): Promise<Plugin> {
  const tuple = await DevupUI(options)
  if (!Array.isArray(tuple)) throw new TypeError('Expected Vite plugin tuple')
  const plugin = await tuple[0]
  if (!plugin || typeof plugin !== 'object' || Array.isArray(plugin))
    throw new TypeError('Expected main plugin')
  plugins.add(plugin)
  return plugin
}
function invoke(plugin: Plugin, name: Hook, args: unknown[] = []): unknown {
  const hook = plugin[name]
  if (!hook) throw new TypeError(`Missing ${name}`)
  return Reflect.apply(
    typeof hook === 'function' ? hook : hook.handler,
    undefined,
    args,
  )
}
const integration = (root = process.cwd()) => ({
  integration: 'lifecycle-test',
  root,
})
const owner = () => runBuildOperation(integration(), () => 42)
const mixed = expect.objectContaining({ name: 'MixedBuildIntegrationError' })
const blocked = () => expect(owner).toThrow(mixed)
function input(root: string, plugins: Plugin[] = []): InlineConfig {
  return { configFile: false, root, plugins }
}
const { getPrefix, isDebug, exportFileMap, exportSheet } = wasm
const state = () => [getPrefix(), isDebug(), exportFileMap(), exportSheet()]
const readTheme = (plugin: Plugin) => invoke(plugin, 'config', [{}])
const themeKey = 'process.env.DEVUP_UI_DEFAULT_THEME'
const shorthands = { customInset: ['left', 'right'] }

it('keeps raw factories inert when legacy state or an owned operation is live', async () => {
  // Given a real selected legacy configuration with nondefault native engine options.
  const root = fixture()
  const selected = await main({ prefix: 'retained', debug: true, shorthands })
  await resolveConfig(input(root, [selected]), 'serve')
  const before = state()
  // When a discarded factory is created and closed beside the selected plugin.
  const discarded = await main({ prefix: 'discarded', debug: false })
  invoke(discarded, 'closeBundle')
  // Then configuration survives and the genuinely live legacy interval still blocks owners.
  expect(state()).toEqual(before)
  blocked()
  invoke(selected, 'closeBundle')
  await runBuildOperation(integration(root), () => main())
  expect(owner()).toBe(42)
})

it.each([
  ['serve', true, true],
  ['serve', true, false],
  ['build', false, true],
  ['build', false, false],
] as const)(
  'preserves repeated %s configuration and reacquires after close (extract=%s, config-first=%s)',
  async (command, extractCss, configFirst) => {
    // Given real theme configuration and either legal initial hook order.
    const root = fixture()
    writeFileSync(
      join(root, 'devup.json'),
      JSON.stringify({ theme: { colors: { dark: { text: '#fff' } } } }),
    )
    const plugin = await main({ extractCss })
    const config = await resolveConfig(input(root), command)
    if (configFirst) invoke(plugin, 'config', [{}])
    await invoke(plugin, 'configResolved', [config])
    wasm.seedFileMap(['retained.tsx'])
    const map = wasm.exportFileMap()
    // When configuration repeats and reads its real registered theme.
    await invoke(plugin, 'configResolved', [config])
    expect(readTheme(plugin)).toMatchObject({
      define: { [themeKey]: '"dark"' },
    })
    // Then numbering remains append-only and close/reuse starts another genuine interval.
    expect(wasm.exportFileMap()).toBe(map)
    blocked()
    invoke(plugin, 'closeBundle')
    invoke(plugin, 'closeBundle')
    expect(owner()).toBe(42)
    await invoke(plugin, 'configResolved', [config])
    blocked()
    invoke(plugin, 'closeWatcher')
    invoke(plugin, 'closeWatcher')
    expect(owner()).toBe(42)
  },
)

it('cold-reads once without caching later positive theme defines', async () => {
  // Given a previous owner theme, written after construction to distinguish deferred reset.
  const plugin = await main()
  wasm.registerTheme({ colors: { previous: { text: '#000' } } })
  // When the first config read enters and releases its short native interval.
  expect(readTheme(plugin)).toMatchObject({ define: {} })
  expect(owner()).toBe(42)
  wasm.registerTheme({ colors: { later: { text: '#fff' } } })
  // Then later positive reads use the live getter rather than a cached empty value.
  expect(readTheme(plugin)).toMatchObject({ define: { [themeKey]: '"later"' } })
  expect(owner()).toBe(42)
  invoke(plugin, 'closeBundle')
})

it('releases only its own interval when the theme getter or selected setter throws', async () => {
  // Given a real live legacy plugin and a second plugin with a failing setup boundary.
  const root = fixture()
  const selected = await main()
  const failed = await main()
  const config = await resolveConfig(input(root, [selected]), 'serve')
  const getter = spyOn(wasm, 'getDefaultTheme').mockImplementation(() => {
    throw new TypeError('getter fault')
  })
  try {
    expect(() => invoke(failed, 'config', [{}])).toThrow('getter fault')
  } finally {
    getter.mockRestore()
  }
  const setter = spyOn(wasm, 'setPrefix').mockImplementation(() => {
    throw new TypeError('setter fault')
  })
  try {
    await expect(
      Promise.resolve(invoke(failed, 'configResolved', [config])),
    ).rejects.toThrow('setter fault')
  } finally {
    setter.mockRestore()
  }
  invoke(failed, 'closeBundle')
  invoke(failed, 'closeWatcher')
  // Then the selected lease survives each failure and only its own close permits admission.
  blocked()
  invoke(selected, 'closeBundle')
  expect(owner()).toBe(42)
})

it('releases its selected legacy lease when reused for production configuration', async () => {
  // Given a configured serve plugin subsequently selected for an extracting build.
  const root = fixture()
  const plugin = await main()
  await resolveConfig(input(root, [plugin]), 'serve')
  // When the actual production selection replaces the legacy path.
  await resolveConfig(input(root, [plugin]), 'build')
  // Then no own legacy callback remains to reject the production owner.
  expect(owner()).toBe(42)
})
