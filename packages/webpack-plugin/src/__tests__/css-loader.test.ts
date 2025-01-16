import { getCss } from '@devup-ui/wasm'

import devupUICssLoader from '../css-loader'

vi.mock('@devup-ui/wasm')

describe('devupUICssLoader', () => {
  it('should invoke callback', () => {
    vi.mocked(getCss).mockReturnValue('css')
    const callback = vi.fn()
    devupUICssLoader.bind({
      callback,
    } as any)(Buffer.from(''), '')
    expect(callback).toBeCalledWith(null, 'css')
  })
})
