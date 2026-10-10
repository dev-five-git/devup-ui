import { resolve } from 'node:path'

import { it } from 'bun:test'

import { ownSlotParity } from './mdx-own-slot-parity'

it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((compiler) =>
    [false, true].map((sourceMap) => ({ compiler, sourceMap })),
  ),
)(
  'matches the actual Devup normal entry with $compiler and maps $sourceMap',
  async ({ compiler, sourceMap }) =>
    ownSlotParity(resolve(import.meta.dir, '../../../..'), compiler, sourceMap),
)
