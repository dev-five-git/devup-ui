import assert from 'node:assert/strict'
import { dirname, relative, resolve } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import { expect, it, mock, spyOn } from 'bun:test'

import { createReservation, staticEntries } from '../reservation'

it('collects only static entry requests without executing opaque functions', () => {
  // Given the public Rspack entry variants.
  const opaque = mock(() => {
    throw new Error('must stay opaque')
  })
  // When the adapter inspects them.
  const result = [
    undefined,
    opaque,
    './entry.js',
    ['./a.js'],
    {
      a: './a.js',
      b: ['./b.js'],
      c: { import: './c.js' },
      d: { import: ['./d.js'] },
      e: { import: [] },
    },
  ].map(staticEntries)
  // Then native function evaluation remains exclusively native.
  expect(result).toEqual([
    [],
    [],
    ['./entry.js'],
    ['./a.js'],
    ['./a.js', './b.js', './c.js', './d.js'],
  ])
  expect(opaque).not.toHaveBeenCalled()
})

it.each([false, true])(
  'preserves exact reservation IDs and flags in atomMode=%s',
  (atomMode) => {
    // Given one manager shared by known environment roots.
    const reserve = mock(
      (
        discovery: utils.ReservationDiscoveryOptions,
        batch: utils.ReservationBatchOptions,
      ) => {
        assert(discovery.toId)
        expect(discovery.toId('C:\\project\\app\\A.js')).toBe(
          atomMode ? 'C:/project/app/A.js' : 'C:\\project\\app\\A.js',
        )
        expect(new Set(discovery.roots)).toEqual(
          new Set([
            resolve('src'),
            resolve('app'),
            resolve('other/src'),
            resolve('other/app'),
          ]),
        )
        expect(discovery.entries).toEqual([
          { request: './a.js', context: resolve('external') },
        ])
        expect(discovery).toMatchObject({
          cwd: process.cwd(),
          include: ['included-ui'],
          package: '@devup-ui/react',
          importAliases: {},
        })
        const physicalPath = resolve('external/a.js')
        expect(
          batch.cssDir({ physicalPath, filename: physicalPath, code: '' }),
        ).toBe(
          `./${relative(dirname(physicalPath), resolve('df')).replaceAll('\\', '/')}`,
        )
        expect(batch).toMatchObject({
          singleCss: true,
          importMainCssInCode: atomMode,
          importMainCssInCss: !atomMode,
        })
        return true
      },
    )
    const invalidate = mock()
    const create = spyOn(utils, 'createSharedSlotReservation').mockReturnValue({
      reserve,
      invalidate,
    })
    try {
      const reservation = createReservation({
        root: resolve('other'),
        atomMode,
        package: '@devup-ui/react',
        include: ['included-ui'],
        cssDir: resolve('df'),
        singleCss: true,
        importAliases: {},
      })
      reservation.addEntries('./a.js', resolve('external'))
      reservation.addEntries(undefined)
      // When native compilation commits the whole inventory.
      reservation.invalidate()
      reservation.reserve()
      // Then the same manager receives the exact real extraction contract.
      expect(create).toHaveBeenCalledTimes(1)
      expect(invalidate).toHaveBeenCalledTimes(1)
      expect(reserve).toHaveBeenCalledTimes(1)
    } finally {
      create.mockRestore()
    }
  },
)
