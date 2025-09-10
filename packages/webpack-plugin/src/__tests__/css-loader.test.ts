import { resolve } from 'node:path'

import { getCss } from '@devup-ui/wasm'

import devupUICssLoader from '../css-loader'

vi.mock('node:path')
vi.mock('@devup-ui/wasm')

beforeEach(() => {
  vi.resetAllMocks()
})

describe('devupUICssLoader', () => {
  it('should return css on no watch', () => {
    const callback = vi.fn()
    const addContextDependency = vi.fn()
    vi.mocked(resolve).mockReturnValue('resolved')
    vi.mocked(getCss).mockReturnValue('get css')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      resourcePath: 'devup-ui.css',
      getOptions: () => ({ watch: false }),
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, 'get css')
  })

  it('should return _compiler hit css on watch', () => {
    const callback = vi.fn()
    const addContextDependency = vi.fn()
    vi.mocked(resolve).mockReturnValue('resolved')
    vi.mocked(getCss).mockReturnValue('get css')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, 'get css', '', undefined)
    expect(getCss).toBeCalledTimes(1)
    vi.mocked(getCss).mockReset()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(getCss).toBeCalledTimes(0)

    vi.mocked(getCss).mockReset()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      _compiler: {
        __DEVUP_CACHE: 'data',
      },
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(getCss).toBeCalledTimes(0)
  })
})
