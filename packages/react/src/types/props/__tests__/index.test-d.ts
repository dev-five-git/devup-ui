import type { DevupCommonProps, DevupProps } from '..'

describe('index', () => {
  it('DevupCommonProps', () => {
    assertType<DevupCommonProps>({
      bg: 'red',
      bgColor: 'red',
    })
  })

  it('DevupCommonProps _selector', () => {
    assertType<DevupProps<'div'>>({
      _hover: {
        bg: 'red',
        _active: {
          bg: 'blue',
        },
      },
    })

    assertType<DevupProps<'div'>>({
      _hover: `
      background-color: red;
      `,
    })

    expectTypeOf<DevupProps<'div'>>().toExtend<DevupProps<'div'>['_hover']>()
  })

  it('DevupCommonProps selectors', () => {
    assertType<DevupProps<'div'>>({
      selectors: {
        '&:hover': {
          bg: 'red',
        },
      },
    })
    assertType<DevupProps<'div'>>({
      selectors: {
        '&:hover': `
        background-color: red;
        `,
      },
    })

    assertType<DevupProps<'div'>>({
      selectors: {
        '&:hover': [
          `
        background-color: red;
        `,
          {
            bg: 'blue',
          },
        ],
      },
    })
  })
})
