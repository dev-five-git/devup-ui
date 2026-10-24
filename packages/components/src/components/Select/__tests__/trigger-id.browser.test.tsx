import { describe, expect, it } from 'bun:test'
import { render } from 'bun-test-env-dom'

import { Select, SelectContainer, SelectOption, SelectTrigger } from '../index'

function Menu(props: { readonly id?: string; readonly triggerId?: string }) {
  return (
    <Select id={props.id} open>
      <SelectTrigger {...(props.triggerId ? { id: props.triggerId } : {})}>
        Pick
      </SelectTrigger>
      <SelectContainer>
        <SelectOption value="a">A</SelectOption>
      </SelectContainer>
    </Select>
  )
}

describe('Select trigger ids', () => {
  it('labels each listbox with its own unique trigger when selects have no ids', () => {
    // Given two selects without caller-provided ids.
    // When they are rendered together.
    const { container } = render(
      <>
        <Menu />
        <Menu />
      </>,
    )
    const triggers = container.querySelectorAll('[aria-haspopup=listbox]')
    const listboxes = container.querySelectorAll('[role=listbox]')

    // Then their nonempty trigger ids are distinct and label their own listboxes.
    expect(triggers).toHaveLength(2)
    expect(listboxes).toHaveLength(2)
    expect(triggers[0]?.id).toBeTruthy()
    expect(triggers[1]?.id).toBeTruthy()
    expect(triggers[0]?.id).not.toBe(triggers[1]?.id)
    expect(listboxes[0]).toHaveAttribute('aria-labelledby', triggers[0]?.id)
    expect(listboxes[1]).toHaveAttribute('aria-labelledby', triggers[1]?.id)
  })

  it('uses the select id for its trigger and listbox label when an id is provided', () => {
    // Given a select with an explicit root id.
    // When it is rendered open.
    const { container } = render(<Menu id="x" />)

    // Then the trigger and listbox label use that id as their base.
    expect(container.querySelector('[aria-haspopup=listbox]')?.id).toBe(
      'x-trigger',
    )
    expect(container.querySelector('[role=listbox]')).toHaveAttribute(
      'aria-labelledby',
      'x-trigger',
    )
  })

  it('keeps the explicit trigger id as the listbox label when both ids are provided', () => {
    // Given different explicit root and trigger ids.
    // When the select is rendered open.
    const { container } = render(<Menu id="x" triggerId="custom-trigger" />)

    // Then the explicit trigger id wins over the derived default.
    expect(container.querySelector('[aria-haspopup=listbox]')).toHaveAttribute(
      'id',
      'custom-trigger',
    )
    expect(container.querySelector('[role=listbox]')).toHaveAttribute(
      'aria-labelledby',
      'custom-trigger',
    )
  })
})
