import { describe, expect, it, mock } from 'bun:test'
import { act, fireEvent, render, userEvent } from 'bun-test-env-dom'

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

  it('is a switch toggled by Space and Enter', () => {
    const onChange = mock()
    const { getByRole, container } = render(
      <Toggle aria-label="Dark mode" name="dark" onChange={onChange} />,
    )
    const toggle = getByRole('switch')
    expect(toggle).toHaveAttribute('aria-checked', 'false')
    expect(toggle).toHaveAttribute('aria-label', 'Dark mode')
    expect(toggle).toHaveAttribute('tabindex', '0')
    fireEvent.keyDown(toggle, { key: ' ' })
    expect(onChange).toHaveBeenLastCalledWith(true)
    expect(toggle).toHaveAttribute('aria-checked', 'true')
    fireEvent.keyDown(toggle, { key: 'Enter' })
    expect(onChange).toHaveBeenLastCalledWith(false)
    fireEvent.keyDown(toggle, { key: 'a' })
    expect(onChange).toHaveBeenCalledTimes(2)
    expect(container.querySelector('input')).toHaveAttribute('name', 'dark')
  })

  it('ignores keys and leaves the tab order when disabled', () => {
    const onChange = mock()
    const { getByRole, container } = render(
      <Toggle aria-labelledby="l" disabled onChange={onChange} />,
    )
    const toggle = getByRole('switch')
    expect(toggle).toHaveAttribute('tabindex', '-1')
    expect(toggle).toHaveAttribute('aria-labelledby', 'l')
    fireEvent.keyDown(toggle, { key: ' ' })
    expect(onChange).not.toHaveBeenCalled()
    expect(container.querySelector('input')).toBeDisabled()
  })
})
