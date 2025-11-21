import type { AdditionalGlobalCssProps, GlobalCssProps } from '../global-css'

describe('globalCss', () => {
  it('globalCss', () => {
    assertType<GlobalCssProps | AdditionalGlobalCssProps>({
      imports: ['https://example.com'],
      a: {
        color: 'blue',
      },
      _hover: {
        bg: 'red',
      },
    })
  })
})
