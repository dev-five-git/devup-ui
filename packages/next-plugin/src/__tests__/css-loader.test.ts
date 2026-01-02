import { beforeEach, describe, expect, it, mock } from 'bun:test'

const mockGetCss = mock(() => 'get css')
const mockRegisterTheme = mock()

mock.module('@devup-ui/wasm', () => ({
  registerTheme: mockRegisterTheme,
  getCss: mockGetCss,
}))

import devupUICssLoader from '../css-loader'

beforeEach(() => {
  mockGetCss.mockReset()
  mockRegisterTheme.mockReset()

  mockGetCss.mockReturnValue('get css')
})

describe('devupUICssLoader', () => {
  it('should return css on no watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      resourcePath: 'devup-ui.css',
      getOptions: () => ({ watch: false }),
    } as any)(Buffer.from('data'), '')
    expect(callback).toHaveBeenCalledWith(
      null,
      Buffer.from('data'),
      '',
      undefined,
    )
  })

  it('should return _compiler hit css on watch', () => {
    const callback = mock()
    const addContextDependency = mock()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')
    expect(callback).toHaveBeenCalledWith(null, 'get css', '', undefined)
    expect(mockGetCss).toHaveBeenCalledTimes(1)
    mockGetCss.mockReset()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(mockGetCss).toHaveBeenCalledTimes(1)

    mockGetCss.mockReset()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(mockGetCss).toHaveBeenCalledTimes(1)
  })
})
