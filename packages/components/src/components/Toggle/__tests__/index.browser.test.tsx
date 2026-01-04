import { describe, expect, it, mock } from 'bun:test'
import { act, render, userEvent } from 'bun-test-env-dom'

import { Toggle } from '../index'

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
          className="test-toggle-wrapper  "
          classNames={{ toggle: 'test-toggle' }}
          value={true}
          variant="switch"
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <Toggle
          style={{
            backgroundColor: 'blue',
          }}
          styles={{
            toggle: {
              backgroundColor: 'blue',
            },
          }}
          value={true}
          variant="switch"
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
    const onChange = mock()
    const { container } = render(
      <Toggle className="test" onChange={onChange} />,
    )
    const toggleButton = container.querySelector('.test')
    const input = container.querySelector('input')
    toggleButton &&
      (await act(async () => {
        await userEvent.click(toggleButton)
      }))
    expect(input).toHaveAttribute('value', 'true')
  })
})
