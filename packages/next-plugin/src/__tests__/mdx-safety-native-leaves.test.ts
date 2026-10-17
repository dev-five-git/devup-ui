import { expect, it } from 'bun:test'

import { compileMdx } from '../mdx-prepare'
import { fixture, type RemarkTree } from './mdx-boundary-fixture.test'
import { withNativeMdx } from './mdx-safety-native'

it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((entry) =>
    (['cycle', 'accessor'] as const).map((form) => ({ entry, form })),
  ),
)(
  'matches native default Date serialization with an ignored own $form through $entry',
  async ({ entry, form }) => {
    // Given
    const date = new Date('2026-01-01T00:00:00.000Z')
    let effects = 0
    switch (form) {
      case 'cycle':
        Reflect.set(date, 'self', date)
        break
      case 'accessor':
        Object.defineProperty(date, 'ignored', {
          enumerable: true,
          get() {
            effects++
            return 'ignored'
          },
        })
        break
      default:
        return form satisfies never
    }
    const seen: Date[] = []
    const attacher = (options: { readonly date: Date }) => {
      seen.push(options.date)
      return (tree: RemarkTree) => {
        const node = tree.children[0]?.children[0]
        if (node) node.value = String(options.date.getUTCFullYear())
      }
    }
    const options = { format: 'mdx', remarkPlugins: [[attacher, { date }]] }
    const hashInput = JSON.stringify(options)
    const request = fixture(entry, options)
    await withNativeMdx(request, async (native) => {
      const expected = await native.compile()
      expect(expected.source).toContain('2026')
      expect(effects).toBe(0)
      // When
      const output = await compileMdx({
        ...request,
        context: { ...request.context, compiler: native.compiler },
      })
      // Then
      expect(output.source).toBe(expected.source)
      expect(JSON.stringify(options)).toBe(hashInput)
      expect(seen).toHaveLength(2)
      expect(seen[0]).toBe(date)
      expect(seen[1]).toBe(date)
      expect(effects).toBe(0)
    })
  },
)

it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((entry) => [
    { entry, label: 'Date', leaf: new Date('2026-01-01T00:00:00.000Z') },
    { entry, label: 'RegExp', leaf: /ordinary-leaf/ },
  ]),
)(
  'preserves native $label plugin leaves with $entry',
  async ({ entry, leaf }) => {
    // Given
    const seen: unknown[] = []
    const attacher = (options: { readonly leaf: Date | RegExp }) => {
      seen.push(options.leaf)
      return (tree: RemarkTree) => {
        const node = tree.children[0]?.children[0]
        if (node) node.value = String(options.leaf)
      }
    }
    const request = fixture(entry, {
      format: 'mdx',
      remarkPlugins: [[attacher, { leaf }]],
    })
    await withNativeMdx(request, async (native) => {
      const expected = await native.compile()
      // When
      const output = await compileMdx({
        ...request,
        context: { ...request.context, compiler: native.compiler },
      })
      // Then
      expect(output.source).toBe(expected.source)
      expect(seen).toHaveLength(2)
      expect(seen[0]).toBe(leaf)
      expect(seen[1]).toBe(leaf)
    })
  },
)
