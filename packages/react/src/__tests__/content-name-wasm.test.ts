import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { expect, it } from 'bun:test'

const fixture = fileURLToPath(
  new URL(
    '../../../../bindings/devup-ui-wasm/tests/content-names.mjs',
    import.meta.url,
  ),
)

it.each(['goldens', 'cache-conflict'] as const)(
  'public WASM content naming: %s',
  (mode) => {
    // Given / When: a fresh Node process uses the real packaged WASM, not spies.
    const output = execFileSync('node', [fixture, mode], {
      encoding: 'utf8',
      timeout: 60_000,
    })
    // Then: the fixture's observable-output assertions all passed.
    expect(output).toBe('')
  },
)
