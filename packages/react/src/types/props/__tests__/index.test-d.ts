import { Property } from 'csstype'

import type { ResponsiveValue } from '../../responsive-value'
import type { DevupCommonProps, DevupComponentProps, DevupProps } from '..'
import type { Selectors } from '../selector'

describe('index', () => {
  it('DevupCommonProps', () => {
    assertType<DevupCommonProps>({
      bg: 'red',
      bgColor: 'red',
    })
  })

  it('DevupProps', () => {
    expectTypeOf<DevupProps>()
      .toHaveProperty('bg')
      .toEqualTypeOf<ResponsiveValue<Property.Background>>()
  })

  it('Selectors', () => {
    expectTypeOf<Selectors>().toHaveProperty('&:hover')
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
