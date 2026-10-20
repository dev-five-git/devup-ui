import { dirname, relative, resolve } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, mock, spyOn } from 'bun:test'

import { DevupUIWebpackPlugin } from '../plugin'
import { createWebpackReservation } from '../reservation'

it('maps the physical inventory to the exact loader IDs, CSS directory and flags', () => {
  const input = {
    physicalPath: resolve('physical', 'A.js'),
    filename: 'physical/A.js',
    code: 'source',
  }
  const reserve = mock<utils.SharedSlotReservation['reserve']>(
    (discovery, options) => {
      expect(discovery.roots).toEqual([
        resolve('src'),
        resolve('app'),
        resolve('context', 'src'),
        resolve('context', 'app'),
      ])
      expect(discovery.toId?.(input.physicalPath)).toBe('../physical/A.js')
      expect(discovery.include).toEqual(['@slot/ui'])
      expect(discovery.package).toBe('@devup-ui/react')
      expect(options.singleCss).toBe(true)
      expect(options.importMainCssInCode).toBe(false)
      expect(options.importMainCssInCss).toBe(true)
      expect(options.cssDir(input)).toBe(
        `./${relative(dirname(input.physicalPath), resolve('df', 'devup-ui')).replaceAll('\\', '/')}`,
      )
      return true
    },
  )
  const manager = { invalidate: mock(), reserve }
  const create = spyOn(utils, 'createSharedSlotReservation').mockReturnValue(
    manager,
  )
  const setResolver = spyOn(wasm, 'setModuleResolver').mockReturnValue(
    undefined,
  )
  try {
    const plugin = new DevupUIWebpackPlugin({
      include: ['@slot/ui'],
      singleCss: true,
    })
    const barrier = createWebpackReservation(plugin.options, {})
    barrier({
      context: resolve('context'),
      entry: () => {
        throw new Error('entry must stay opaque')
      },
    })
    expect(create).toHaveBeenCalledWith({
      reserveClassSlots: wasm.reserveClassSlots,
    })
    expect(manager.invalidate).toHaveBeenCalledTimes(1)
    expect(reserve).toHaveBeenCalledTimes(1)
    expect(setResolver).not.toHaveBeenCalled()
  } finally {
    create.mockRestore()
    setResolver.mockRestore()
  }
})
