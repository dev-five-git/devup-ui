import * as wasm from '@devup-ui/wasm'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'

import devupUICssLoader from '../css-loader'

let getCssSpy: ReturnType<typeof spyOn>
let registerThemeSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('get css')
  registerThemeSpy = spyOn(wasm, 'registerTheme').mockReturnValue(undefined)
})

afterEach(() => {
  getCssSpy.mockRestore()
  registerThemeSpy.mockRestore()
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
    expect(getCssSpy).toHaveBeenCalledTimes(1)
    getCssSpy.mockClear()
    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui.css',
    } as any)(Buffer.from('data'), '')

    expect(getCssSpy).toHaveBeenCalledTimes(1)

    getCssSpy.mockClear()

    devupUICssLoader.bind({
      callback,
      addContextDependency,
      getOptions: () => ({ watch: true }),
      resourcePath: 'devup-ui-10.css',
    } as any)(Buffer.from(''), '')

    expect(getCssSpy).toHaveBeenCalledTimes(1)
  })
})
