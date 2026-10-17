import { tmpdir } from 'node:os'
import { dirname } from 'node:path'

import { expect, it } from 'bun:test'

import { sourceFixture } from './mdx-source-fixture'

it('creates compiler fixtures under the platform temporary directory', () => {
  // Given the installed compiler fixture and the current platform's temporary root.
  const root = tmpdir()

  // When the fixture creates its isolated project.
  const fixture = sourceFixture({ 'app/page.tsx': 'export default () => null' })

  // Then it is independent of a particular user's Windows workspace.
  expect(dirname(fixture.root)).toBe(root)
})
