import { spawnSync } from 'node:child_process'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'

/** The ordinary props the fixture spreads beside its `$` props */
const ordinary = { 'data-testid': 'target', id: 'target', title: 'Target' }

const root = new URL('../../../../../', import.meta.url)
const fixture = new URL('fixtures/styled-transient-props.jsx', import.meta.url)

// The fixture renders in a fresh Bun process: in the suite's process, earlier
// test files leave `console.error` replaced by a mock that keeps their calls.
// A script gets no test preload, so the Devup UI plugin is preloaded here.
const child = spawnSync(
  process.execPath,
  [
    '--preload',
    fileURLToPath(new URL('packages/bun-plugin/src/index.ts', root)),
    fileURLToPath(fixture),
  ],
  { cwd: fileURLToPath(root), encoding: 'utf8' },
)

/** What the fixture observed in `scenario`, once its process exited cleanly */
function observation(scenario) {
  expect({ status: child.status, stderr: child.stderr }).toEqual({
    status: 0,
    stderr: '',
  })
  return JSON.parse(child.stdout)[scenario]
}

function transientKeys(props) {
  return Object.keys(props).filter((key) => key.startsWith('$'))
}

describe('styled transient props', () => {
  it('keeps unread $ props of a runtime spread off the tag while its styles read $tone into a CSS variable', () => {
    const { html, warnings } = observation('tag')

    expect(warnings).toEqual([])
    expect(html).not.toMatch(/ \$[^\s=]*=/)
    expect(html).toContain(' data-testid="target"')
    expect(html).toContain(' id="target"')
    expect(html).toContain(' title="Target"')
    expect(html).toMatch(/ style="--[^"]*:teal"/)
  })

  it('passes a component target the ordinary props of a runtime spread but no $ props', () => {
    const { received } = observation('component')

    expect(received).toEqual([expect.objectContaining(ordinary)])
    expect(received.map(transientKeys)).toEqual([[]])
  })

  it('keeps $ props off the tag even when shouldForwardProp forwards every prop', () => {
    const { html, warnings } = observation('forwarding')

    expect(warnings).toEqual([])
    expect(html).not.toMatch(/ \$[^\s=]*=/)
    expect(html).toContain(' id="target"')
  })

  it('keeps $ props that attrs create off the tag', () => {
    const { html, warnings } = observation('attrs')

    expect(warnings).toEqual([])
    expect(html).not.toMatch(/ \$[^\s=]*=/)
    expect(html).toContain(' id="target"')
  })

  it('passes the component `as` names the ordinary props of a runtime spread but no $ props', () => {
    const { received } = observation('as')

    expect(received).toEqual([expect.objectContaining(ordinary)])
    expect(received.map(transientKeys)).toEqual([[]])
  })
})
