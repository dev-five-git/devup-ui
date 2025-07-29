import type { DevupCommonProps, DevupComponentProps } from '..'

describe('index', () => {
  it('DevupCommonProps', () => {
    assertType<DevupCommonProps>({
      bg: 'red',
      bgColor: 'red',
    })
  })

  it('DevupCommonProps _selector', () => {
    assertType<DevupComponentProps<'div'>>({
      _hover: {
        bg: 'red',
        _active: {
          bg: 'blue',
        },
      },
    })

    assertType<DevupComponentProps<'div'>>({
      _hover: `
      background-color: red;
      `,
    })

    expectTypeOf<DevupComponentProps<'div'>>().toExtend<
      DevupComponentProps<'div'>['_hover']
    >()
  })

  it('DevupCommonProps selectors', () => {
    assertType<DevupComponentProps<'div'>>({
      selectors: {
        '&:hover': {
          bg: 'red',
        },
      },
    })
    assertType<DevupComponentProps<'div'>>({
      selectors: {
        '&:hover': `
        background-color: red;
        `,
      },
    })

    assertType<DevupComponentProps<'div'>>({
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
  it('DevupSelectorProps', () => {
    assertType<DevupComponentProps<'div'>>({
      _hover: {
        bg: 'red',
      },
      selectors: {
        '&:hover': {
          bg: 'red',
        },
      },
    })
    assertType<DevupComponentProps<'div'>>({
      selectors: {
        '&:hover': `
        background-color: red;
        `,
      },
      _backdrop: {
        bg: 'red',
      },
    })

    assertType<DevupComponentProps<'div'>>({
      _hover: `
      background-color: red;
      `,
      _backdrop: `
      backdrop-filter: blur(10px);
      `,
    })
  })
})
