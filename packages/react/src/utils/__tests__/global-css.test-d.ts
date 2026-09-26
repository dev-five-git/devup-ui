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

  it('top-level at-rules', () => {
    assertType<GlobalCssProps>({
      '@media (prefers-reduced-motion: reduce)': {
        '*, *::before, *::after': { transition: 'none' },
      },
      _motionReduce: { html: { scrollBehavior: 'auto' } },
      _print: { body: { bg: 'white' } },
      _media: { '(min-width: 768px)': { body: { m: 2 } } },
      '@supports': { '(display: grid)': { main: { display: 'grid' } } },
      body: { _motionReduce: { transition: 'none' } },
    })
  })
})
