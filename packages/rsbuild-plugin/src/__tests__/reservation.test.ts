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
  const entries: Parameters<typeof staticEntries>[0][] = [
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
    { f: { runtime: false as const } },
  ]
  // When the adapter inspects them.
  const result = entries.map(staticEntries)
  // Then native function evaluation remains exclusively native.
  expect(result).toEqual([
    [],
    [],
    ['./entry.js'],
    ['./a.js'],
    ['./a.js', './b.js', './c.js', './d.js'],
    [],
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
          cwd: resolve('other'),
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
      reservation.trackCompiler('web', {
        options: { entry: './a.js', context: resolve('external') },
      })
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

it('replaces compiler revisions while reading the current union of all owners', () => {
  // Given two known environments and an obsolete compiler for one owner.
  const reserve = mock<
    (
      discovery: utils.ReservationDiscoveryOptions,
      batch: utils.ReservationBatchOptions,
    ) => boolean
  >(() => true)
  const create = spyOn(utils, 'createSharedSlotReservation').mockReturnValue({
    reserve,
    invalidate: mock(),
  })
  try {
    const reservation = createReservation({
      root: resolve('project'),
      atomMode: false,
      package: '@devup-ui/react',
      include: [],
      cssDir: resolve('df'),
      singleCss: true,
      importAliases: {},
    })
    const web: { options: { entry: string; context?: string } } = {
      options: { entry: './early.js' },
    }
    reservation.trackCompiler('web', web)
    reservation.trackCompiler('node', {
      options: { entry: { main: { import: ['./node.js'] } } },
    })
    reservation.reserve()
    web.options = { entry: './final.js', context: resolve('external') }
    // When a later compiler boundary reserves changed options without re-registering.
    reservation.reserve()
    // Then the current union replaces the obsolete snapshot, including its context.
    expect(reserve.mock.calls[1]?.[0].entries).toEqual([
      { request: './final.js', context: resolve('external') },
      { request: './node.js', context: resolve('project') },
    ])
    reservation.trackCompiler('web', { options: { entry: './replacement.js' } })
    reservation.reserve()
    expect(reserve.mock.calls[2]?.[0].entries).toEqual([
      { request: './replacement.js', context: resolve('project') },
      { request: './node.js', context: resolve('project') },
    ])
    const opaque = mock(() => ({ main: ['./opaque.js'] }))
    reservation.trackCompiler('web', { options: { entry: opaque } })
    reservation.reserve()
    expect(reserve.mock.calls[3]?.[0].entries).toEqual([
      { request: './node.js', context: resolve('project') },
    ])
    expect(opaque).not.toHaveBeenCalled()
  } finally {
    create.mockRestore()
  }
})
