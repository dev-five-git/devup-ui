import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'

import { readWebpackLoaderBytes } from '../webpack-mdx-bytes'
import { createWebpackLoaderFacts } from '../webpack-mdx-facts'
import {
  extraction,
  mdxLoader,
  mdxRule,
  source,
  withSelector,
} from './webpack-resource-fixture'

describe('native fs cancellation across signal realms', () => {
  it('returns exact native bytes with the supplied realm signal', async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'w21f-resource-bytes-'))
    const filename = join(root, 'loader.js')
    const bytes = Buffer.from('export default "original"')
    writeFileSync(filename, bytes)
    try {
      // When
      const result = await readWebpackLoaderBytes(
        filename,
        new AbortController().signal,
      )
      // Then
      expect(result).toEqual(bytes)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })

  it('preserves pre-abort reason instead of attempting to open a missing file', async () => {
    // Given
    const controller = new AbortController()
    const reason = new Error('cancelled before open')
    controller.abort(reason)
    // When / Then
    await expect(
      readWebpackLoaderBytes('missing-before-abort', controller.signal),
    ).rejects.toBe(reason)
  })

  it('destroys an in-flight native read and preserves the caller abort reason', async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'w21f-resource-abort-'))
    const filename = join(root, 'loader.js')
    writeFileSync(filename, Buffer.alloc(1024 * 1024, 65))
    const controller = new AbortController()
    const reason = new Error('cancelled during native read')
    try {
      // When
      const result = readWebpackLoaderBytes(filename, controller.signal)
      controller.abort(reason)
      // Then
      await expect(result).rejects.toBe(reason)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })

  it('propagates a native file error without converting it to cancellation', async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'w21f-resource-missing-'))
    try {
      // When / Then
      await expect(
        readWebpackLoaderBytes(
          join(root, 'missing.js'),
          new AbortController().signal,
        ),
      ).rejects.toMatchObject({ code: 'ENOENT' })
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })

  it('retries loader facts after the first caller cancels native resolution', async () => {
    // Given
    const result = await withSelector(
      { module: { rules: [mdxRule()] } },
      async (_selector, compiler, binding) => {
        const facts = createWebpackLoaderFacts(binding, 'next.config.mjs')
        const controller = new AbortController()
        const reason = new Error('cancelled native resolver result')
        compiler.resolverFactory
          .get('loader')
          .hooks.result.tap('W21fCancelOnce', () => {
            controller.abort(reason)
          })
        await expect(
          facts.resolveLoader({ loader: mdxLoader }, controller.signal),
        ).rejects.toBe(reason)
        // When
        return facts.resolveLoader(
          { loader: mdxLoader },
          new AbortController().signal,
        )
      },
    )
    // Then
    expect(result.facts.resolvedPath).toBe(mdxLoader)
  })

  it('does not publish ordinary disk-first eligibility when qualification is cancelled', async () => {
    // Given
    const filename = source.replace(/\.mdx$/, '.tsx')
    const config = {
      module: {
        rules: [{ test: /\.tsx$/, enforce: 'pre' as const, use: [extraction] }],
      },
    }
    const result = await withSelector(config, async (selector) => {
      const controller = new AbortController()
      const reason = new Error('cancelled ordinary qualification')
      // When
      const pending = selector.qualifyOrdinary(filename, controller.signal)
      controller.abort(reason)
      // Then
      await expect(pending).rejects.toBe(reason)
      return selector.ordinaryEligibility(filename)
    })
    expect(result.kind).toBe('native-required')
  })
})
