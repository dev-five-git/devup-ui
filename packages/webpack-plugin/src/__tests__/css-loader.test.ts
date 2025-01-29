import { resolve } from 'node:path'

import devupUICssLoader from '../css-loader'

vi.mock('node:path')

describe('devupUICssLoader', () => {
  it('should invoke callback', () => {
    const callback = vi.fn()
    const addContextDependency = vi.fn()
    vi.mocked(resolve).mockReturnValue('resolved')
    devupUICssLoader.bind({
      callback,
      addContextDependency,
    } as any)(Buffer.from('data'), '')
    expect(callback).toBeCalledWith(null, Buffer.from('data'))
    expect(addContextDependency).toBeCalledWith('resolved')
  })
})
