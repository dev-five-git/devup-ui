import { describe, expect, it } from 'bun:test'

import type { DevupConfig, DevupTheme } from '../types'

describe('DevupTheme', () => {
  it('spells the shadow tokens as the docs and the engine do', () => {
    const shadow = { default: { card: '0 1px 2px #0003' } }
    const documented: DevupTheme = { shadow }
    const alias: DevupTheme = { shadows: shadow }
    const config: DevupConfig = { theme: { shadow } }

    expect(documented.shadow).toBe(shadow)
    expect(alias.shadows).toBe(shadow)
    expect(config.theme?.shadow).toBe(shadow)
  })
})
