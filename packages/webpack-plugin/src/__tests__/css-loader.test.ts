import { resolve } from 'node:path'

import { getCss } from '@devup-ui/wasm'
import type { Mock } from 'bun:test'
import { beforeEach, describe, expect, it, mock } from 'bun:test'

import devupUICssLoader from '../css-loader'

mock.module('node:path', () => ({
  resolve: mock(),
}))
mock.module('@devup-ui/wasm', () => ({
  registerTheme: mock(),
  getCss: mock(),
}))

beforeEach(() => {
  ;(resolve as Mock<typeof resolve>).mockReset()
  ;(getCss as Mock<typeof getCss>).mockReset()
})

describe('devupUICssLoader', () => {
  it('should return css on no watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    ;(resolve as Mock<typeof resolve>).mockReturnValue('resolved')
    ;(getCss as Mock<typeof getCss>).mockReturnValue('get css')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      resourcePath: 'devup-ui.css',
      getOptions: () => ({ watch: false }),
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
  })

  it('should return _compiler hit css on watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    ;(resolve as Mock<typeof resolve>).mockReturnValue('resolved')
    ;(getCss as Mock<typeof getCss>).mockReturnValue('get css')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
    expect(getCss).toBeCalledTimes(1)
    ;(getCss as Mock<typeof getCss>).mockReset()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(getCss).toBeCalledTimes(1)
    ;(getCss as Mock<typeof getCss>).mockReset()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(getCss).toBeCalledTimes(1)
  })
})
