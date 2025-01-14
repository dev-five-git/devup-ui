import { DevupUIWebpackPlugin } from '@devup-ui/webpack-plugin'

import { DevupUI } from '../plugin'

vi.mock('@devup-ui/webpack-plugin')

describe('plugin', () => {
  it('should apply webpack plugin', async () => {
    const ret = DevupUI({})

    ret.webpack!({ plugins: [] }, {} as any)

    expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({})
  })

  it('should apply webpack plugin with config', async () => {
    const ret = DevupUI(
      {},
      {
        package: 'new-package',
      },
    )

    ret.webpack!({ plugins: [] }, {} as any)

    expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
      package: 'new-package',
    })
  })

  it('should apply webpack plugin with webpack obj', async () => {
    const webpack = vi.fn()
    const ret = DevupUI(
      {
        webpack,
      },
      {
        package: 'new-package',
      },
    )

    ret.webpack!({ plugins: [] }, {} as any)

    expect(DevupUIWebpackPlugin).toHaveBeenCalledWith({
      package: 'new-package',
    })
    expect(webpack).toHaveBeenCalled()
  })
})
