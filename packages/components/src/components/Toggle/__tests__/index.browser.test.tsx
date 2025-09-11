import { act, render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { Toggle } from '../index'

vi.mock('react', async (originImport: any) => {
  const origin = await originImport()
  return {
    ...origin,
    cache: vi.fn((arg) => arg),
  }
})
describe('Toggle', () => {
  it('should Toggle snapshot', () => {
    expect(render(<Toggle />).container).toMatchSnapshot()
    expect(render(<Toggle disabled />).container).toMatchSnapshot()
    expect(render(<Toggle defaultValue={true} />).container).toMatchSnapshot()
    expect(render(<Toggle value={true} />).container).toMatchSnapshot()
    expect(render(<Toggle variant="switch" />).container).toMatchSnapshot()
    expect(
      render(<Toggle disabled variant="switch" />).container,
    ).toMatchSnapshot()
    expect(
      render(<Toggle defaultValue={true} variant="switch" />).container,
    ).toMatchSnapshot()
    expect(
      render(<Toggle value={true} variant="switch" />).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Toggle
          colors={{
            primary: 'blue',
            bg: 'blue',
            hoverBg: 'blue',
            primaryHoverBg: 'blue',
            disabledBg: 'blue',
            switchHoverOutline: 'blue',
            switchShadow: 'blue',
          }}
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Toggle
          colors={{
            primary: 'blue',
            bg: 'blue',
            hoverBg: 'blue',
            primaryHoverBg: 'blue',
            disabledBg: 'blue',
            switchHoverOutline: 'blue',
            switchShadow: 'blue',
          }}
          variant="switch"
        />,
      ).container,
    ).toMatchSnapshot()
  })

  it('should change value when use onChange prop', async () => {
    const onChange = vi.fn()
    const { container } = render(<Toggle onChange={onChange} />)
    const toggleButton = container.querySelector(`.toggle-switch`)
    const input = container.querySelector('input')
    toggleButton &&
      (await act(async () => {
        await userEvent.click(toggleButton)
      }))
    expect(input).toHaveAttribute('value', 'true')
  })
})
