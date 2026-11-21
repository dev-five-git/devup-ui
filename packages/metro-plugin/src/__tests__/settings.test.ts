import { describe, expect, it } from 'bun:test'

import {
  MissingSettingsError,
  readSettings,
  SETTINGS_ENV,
  writeSettings,
} from '../settings'

const settings = {
  upstreamTransformer: '/upstream.js',
  package: '@devup-ui/react',
  include: ['lib'],
  runtime: false,
  reanimated: true,
  devupFile: '/devup.json',
  engine: 'wasm',
} as const

describe('settings', () => {
  it('round-trips through the environment', () => {
    const env: NodeJS.ProcessEnv = {}
    writeSettings(settings, env)
    expect(JSON.parse(env[SETTINGS_ENV] ?? '')).toEqual(settings)
    expect(readSettings(env)).toEqual(settings)
  })

  it('tells how to configure Metro when withDevupUI() did not run', () => {
    expect(() => readSettings({})).toThrow(MissingSettingsError)
    expect(() => readSettings({})).toThrow(
      'Wrap the Metro config with `withDevupUI(config)`',
    )
  })

  it('reads the process environment by default', () => {
    const previous = process.env[SETTINGS_ENV]
    try {
      writeSettings(settings)
      expect(readSettings()).toEqual(settings)
    } finally {
      if (previous === undefined) delete process.env[SETTINGS_ENV]
      else process.env[SETTINGS_ENV] = previous
    }
  })
})
