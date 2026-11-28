import { describe, expect, it } from 'bun:test'

import * as entry from '../index'
import { DevupUIBuildError } from '../located-error'
import { TRANSFORMER_PATH, withDevupUI } from '../plugin'
import { MissingSettingsError } from '../settings'

describe('@devup-ui/metro-plugin', () => {
  it('exports the plugin, its transformer path and its errors', () => {
    expect(entry.withDevupUI).toBe(withDevupUI)
    expect(entry.TRANSFORMER_PATH).toBe(TRANSFORMER_PATH)
    expect(entry.DevupUIBuildError).toBe(DevupUIBuildError)
    expect(entry.MissingSettingsError).toBe(MissingSettingsError)
    expect(entry.DEFAULT_UPSTREAM_TRANSFORMERS).toEqual([
      '@expo/metro-config/babel-transformer',
      '@react-native/metro-babel-transformer',
    ])
    expect(entry.UpstreamTransformerNotFoundError).toBeFunction()
  })
})
