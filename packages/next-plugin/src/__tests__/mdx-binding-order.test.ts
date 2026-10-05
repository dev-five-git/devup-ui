import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxBinding, MdxBindingError } from '../mdx-binding'
import { type MdxPipeline } from '../mdx-pipeline'
import { compileMdx, createMdxDeadline } from '../mdx-prepare'
import { createWasm } from '../wasm'
import { bindingOwner, bindingReceipt } from './mdx-binding-fixture'

const root = resolve(import.meta.dir, '../../../../apps/landing')
const installed = createRequire(join(root, 'package.json'))

function plugin(options: Readonly<Record<string, number>>) {
  return (tree: unknown) => {
    if (
      typeof tree !== 'object' ||
      tree === null ||
      !('children' in tree) ||
      !Array.isArray(tree.children)
    )
      throw new TypeError('Expected a native MDX tree')
    const color = Object.keys(options)[0]
    if (color === undefined) throw new TypeError('Expected a configured color')
    for (const value of tree.children) {
      const node: unknown = value
      if (
        typeof node === 'object' &&
        node !== null &&
        'type' in node &&
        node.type === 'mdxJsxFlowElement' &&
        'name' in node &&
        node.name === 'Box' &&
        'attributes' in node &&
        Array.isArray(node.attributes)
      )
        node.attributes.push({
          type: 'mdxJsxAttribute',
          name: 'bg',
          value: color,
        })
    }
  }
}

async function css(
  filename: string,
  options: Readonly<Record<string, number>>,
) {
  const pipeline: MdxPipeline = {
    bundler: 'webpack',
    ruleKey: 'order-proof',
    conditions: [],
    aliases: {},
    loaders: [
      {
        loader: installed.resolve('@next/mdx/mdx-js-loader'),
        options: { jsx: true, remarkPlugins: [[plugin, options]] },
      },
    ],
  }
  const compiled = await compileMdx({
    root,
    filename,
    pipeline,
    signal: new AbortController().signal,
    deadline: createMdxDeadline(),
  })
  const engine = createWasm(root)
  engine.seedFileMap([filename])
  const output = engine.codeExtractWithoutSourceMap(
    filename,
    compiled.source,
    '@devup-ui/react',
    './styles',
    true,
    false,
    true,
    {},
  )
  try {
    return engine.getCss(undefined, false)
  } finally {
    output.free()
  }
}

it('rejects a structurally equal option map whose native key order produces different CSS', async () => {
  // Given
  const directory = mkdtempSync(join(tmpdir(), 'devup-binding-order-'))
  const filename = join(directory, 'page.mdx')
  writeFileSync(
    filename,
    'import {Box} from "@devup-ui/react"\n\n<Box>order</Box>',
  )
  const left = { red: 1, blue: 1 }
  const right = { blue: 1, red: 1 }
  const owner = createMdxBinding(bindingOwner)
  owner.accept(bindingReceipt({ remarkPlugins: [[plugin, left]] }))
  const before = owner.state()
  try {
    const first = await css(filename, left)
    const second = await css(filename, right)
    expect(first).toContain('background:red')
    expect(second).toContain('background:blue')
    // When
    const rebind = () =>
      owner.accept(bindingReceipt({ remarkPlugins: [[plugin, right]] }))
    // Then
    expect(rebind).toThrow(MdxBindingError)
    expect(owner.state()).toBe(before)
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})
