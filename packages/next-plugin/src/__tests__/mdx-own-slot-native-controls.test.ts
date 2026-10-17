import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { fixture, originalCompile } from './mdx-boundary-fixture.test'

it.each([
  { normal: 'downstream', message: 'downstream normal executed' },
  { normal: 'compiler', message: 'must not run' },
])(
  'fails at the $normal guard when the native normal chain reaches it',
  async ({ normal, message }) => {
    // Given: an original installed compiler followed by the same guarded normal.
    const request = fixture('@mdx-js/loader', { jsx: true })
    const guard = join(request.root, 'guard.cjs')
    writeFileSync(
      guard,
      `module.exports = require(${JSON.stringify(join(import.meta.dir, 'mdx-own-slot-normals.cjs'))})[${JSON.stringify(normal)}]`,
    )
    // When / Then: the real runner must propagate the guard's failure, not publish bytes.
    await expect(
      originalCompile({
        ...request,
        pipeline: {
          ...request.pipeline,
          loaders: [{ loader: guard }, ...request.pipeline.loaders],
        },
      }),
    ).rejects.toThrow(message)
  },
)

it('preserves raw resource bytes when the native runner executes the shared pass-through normal', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const raw = join(request.root, 'raw.cjs')
  writeFileSync(
    raw,
    `module.exports = { default: require(${JSON.stringify(join(import.meta.dir, 'mdx-own-slot-normals.cjs'))}).default, raw: true }`,
  )
  // When
  const output = await originalCompile({
    ...request,
    pipeline: { ...request.pipeline, loaders: [{ loader: raw }] },
  })
  // Then
  expect(output).toEqual({ source: '# original', map: undefined })
})
