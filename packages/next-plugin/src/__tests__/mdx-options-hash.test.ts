import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { runInNewContext } from 'node:vm'

import { expect, it } from 'bun:test'

import { createMdxOptionsInstance } from '../mdx-options-instance'
import { isMdxRecord } from '../mdx-pipeline'
import { compileMdx } from '../mdx-prepare'
import { prepareMdxPlugins } from '../mdx-prepare-plugins'
import {
  fixture,
  originalCompile,
  type RemarkTree,
} from './mdx-boundary-fixture.test'

const installed = createRequire(
  join(resolve(import.meta.dir, '../../../../apps/landing'), 'package.json'),
)
const mdx = installed.resolve('@mdx-js/loader')
const source = readFileSync(join(mdx, '../lib/index.js'), 'utf8')
const hash: unknown = runInNewContext(
  `(${source.slice(source.indexOf('function getOptionsHash(')).trim()})`,
  { createHash, JSON, Object },
)
if (typeof hash !== 'function')
  throw new TypeError('installed hash unavailable')

it('keeps the installed processor hash unchanged when approved webpack shells are copied', async () => {
  // Given
  const named = createRequire(mdx).resolve('remark-parse')
  const pluginOptions = { enabled: true }
  const options = {
    jsx: true,
    remarkPlugins: [[named, pluginOptions]],
    other: pluginOptions,
  }
  const request = fixture('@next/mdx/mdx-js-loader', options)
  const copied = createMdxOptionsInstance().loadersFor(request.pipeline)[0]
    ?.options
  if (!isMdxRecord(copied)) throw new TypeError('missing copied options')
  const wrapper = installed.resolve('@next/mdx/mdx-js-loader')
  const native = await prepareMdxPlugins(
    { ...options, remarkPlugins: [[named, pluginOptions]] },
    request.root,
    wrapper,
  )
  // When
  const prepared = await prepareMdxPlugins(copied, request.root, wrapper)
  // Then
  expect(hash(prepared.options)).toBe(hash(native.options))
  expect(hash(copied)).toBe(hash({ ...copied }))
  expect(options.remarkPlugins[0]?.[0]).toBe(named)
})

it('reuses the installed processor when another preparation instance changes only container identities', async () => {
  // Given
  const observations: unknown[] = []
  const options = { text: 'same processor' }
  const plugin = (received: typeof options) => {
    observations.push(received)
    return (tree: RemarkTree) => {
      const text = tree.children[0]?.children[0]
      if (text) text.value = received.text
    }
  }
  const request = fixture('@next/mdx/mdx-js-loader', {
    remarkPlugins: [[plugin, options]],
  })
  await compileMdx({ ...request, optionsInstance: createMdxOptionsInstance() })
  // When
  const output = await compileMdx({
    ...request,
    optionsInstance: createMdxOptionsInstance(),
  })
  // Then
  expect(observations).toEqual([options])
  expect(observations[0]).toBe(options)
  expect(output.source).toContain('same processor')
})

it('passes accumulated plugin-option state to the native webpack chain after preparation', async () => {
  // Given
  const options = { count: 0 }
  const plugin = (received: typeof options) => (tree: RemarkTree) => {
    received.count += 1
    const text = tree.children[0]?.children[0]
    if (text) text.value = `native-visit-${received.count}`
  }
  const request = fixture('@next/mdx/mdx-js-loader', {
    remarkPlugins: [[plugin, options]],
  })
  await compileMdx({ ...request, optionsInstance: createMdxOptionsInstance() })
  // When
  const output = await originalCompile(request)
  // Then
  expect(output.source).toContain('native-visit-2')
  expect(options.count).toBe(2)
})
