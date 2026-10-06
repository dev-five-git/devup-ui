import { expect, it } from 'bun:test'

import {
  installMdxNormalCapture,
  type MdxNormalSeam,
} from '../mdx-normal-capture'
import { compileMdx } from '../mdx-prepare'
import { fullFixture } from './mdx-own-slot-fixture'

const seam: MdxNormalSeam = {
  ruleKey: 'boundary-unit',
  compilerPath: 'compiler',
  compilerIndex: 0,
  loaderCount: 1,
  bridge: false,
}

it.each(
  [
    undefined,
    [],
    [undefined],
    [{}],
    [Object.freeze({ normal: undefined })],
  ].map((value) => ({ value })),
)(
  'rejects unsupported assignment facts without publishing for $value',
  ({ value }) => {
    // Given: these are structural boundary negatives, not native-invocation certificates.
    const context = {}
    installMdxNormalCapture(context, seam)
    // When / Then
    expect(() => Reflect.set(context, 'loaders', value)).toThrow()
  },
)

it('rejects a missing callable normal when an assigned loader export has no normal', () => {
  // Given
  const context = {}
  const slot = { normal: undefined }
  installMdxNormalCapture(context, seam)
  Reflect.set(context, 'loaders', [slot])
  // When / Then
  expect(() => Reflect.set(slot, 'normal', undefined)).toThrow()
})

it('locates deadline and callback errors even before a runner assigns loader objects', () => {
  // Given
  const capture = installMdxNormalCapture({}, seam)
  // When / Then
  expect(capture.timeoutError()).toMatchObject({ loader: 'compiler' })
  expect(() => capture.accept('boundary failure', undefined)).toThrow()
  expect(() => capture.accept(null, undefined)).toThrow()
})

it('rejects an unknown original compiler pitch that bypasses its normal', async () => {
  // Given
  const request = fullFixture('return source')
  const directory = join(request.root, '@next/mdx')
  mkdirSync(directory, { recursive: true })
  const loader = join(directory, 'mdx-js-loader.js')
  writeFileSync(
    loader,
    'module.exports = function() { throw new Error("must not run") }; module.exports.pitch = function() { return "bypassed" }',
  )
  const pipeline = { ...request.pipeline, loaders: [{ loader }] }
  // When / Then
  await expect(
    compileMdx({
      root: request.root,
      filename: request.filename,
      signal: request.signal,
      deadline: request.deadline,
      context: request.context,
      pipeline,
    }),
  ).rejects.toMatchObject({
    cause: { fact: 'exactly one compiler execution (pitch bypass)' },
  })
})
import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
