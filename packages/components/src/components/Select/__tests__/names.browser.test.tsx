import { describe, expect, it } from 'bun:test'
import { fireEvent, render } from 'bun-test-env-dom'

import { Select, SelectContainer, SelectOption, SelectTrigger } from '../index'

/** The name a screen reader gives an element: aria-labelledby, then aria-label, then its content */
function nameOf(element: Element): string {
  const labelledBy = element.getAttribute('aria-labelledby')
  if (labelledBy)
    return labelledBy
      .split(' ')
      .map((id) => document.getElementById(id)?.textContent ?? '')
      .join(' ')
  return element.getAttribute('aria-label') ?? element.textContent ?? ''
}

describe('Select accessible names', () => {
  it('names an option by its content, not by a forced label', () => {
    const { container } = render(
      <Select defaultOpen id="s">
        <SelectTrigger>Pick</SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">Apple</SelectOption>
          <SelectOption aria-label="Banana fruit" value="b">
            Banana
          </SelectOption>
        </SelectContainer>
      </Select>,
    )
    const [apple, banana] = Array.from(
      container.querySelectorAll('[role=option]'),
    )

    expect(apple).not.toHaveAttribute('aria-label')
    expect(nameOf(apple)).toBe('Apple')
    expect(nameOf(banana)).toBe('Banana fruit')
  })

  it('names the listbox by its trigger', () => {
    const { container } = render(
      <Select defaultOpen id="s">
        <SelectTrigger>Pick a fruit</SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">Apple</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const listbox = container.querySelector('[role=listbox]')!
    const trigger = container.querySelector('[aria-haspopup=listbox]')!

    expect(listbox).not.toHaveAttribute('aria-label')
    expect(listbox.getAttribute('aria-labelledby')).toBe(trigger.id)
    expect(nameOf(listbox)).toBe('Pick a fruit')
    expect(trigger).not.toHaveAttribute('aria-label')
  })

  it('follows the id the caller gives the trigger, also on a child it renders', () => {
    const own = render(
      <Select defaultOpen>
        <SelectTrigger id="mine">Pick</SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">Apple</SelectOption>
        </SelectContainer>
      </Select>,
    )
    expect(
      own.container
        .querySelector('[role=listbox]')
        ?.getAttribute('aria-labelledby'),
    ).toBe('mine')

    const child = render(
      <Select defaultOpen>
        <SelectTrigger asChild>
          <button id="child" type="button">
            Pick
          </button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">Apple</SelectOption>
        </SelectContainer>
      </Select>,
    )
    expect(
      child.container
        .querySelector('[role=listbox]')
        ?.getAttribute('aria-labelledby'),
    ).toBe('child')
  })

  it('leaves the name of the listbox to the caller who gives one', () => {
    const labelled = render(
      <Select defaultOpen id="s">
        <SelectTrigger>Pick</SelectTrigger>
        <SelectContainer aria-label="Fruits">
          <SelectOption value="a">Apple</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const listbox = labelled.container.querySelector('[role=listbox]')!
    expect(listbox).not.toHaveAttribute('aria-labelledby')
    expect(nameOf(listbox)).toBe('Fruits')

    const by = render(
      <>
        <h2 id="title">Fruit list</h2>
        <Select defaultOpen id="t">
          <SelectTrigger>Pick</SelectTrigger>
          <SelectContainer aria-labelledby="title">
            <SelectOption value="a">Apple</SelectOption>
          </SelectContainer>
        </Select>
      </>,
    )
    expect(
      by.container
        .querySelector('[role=listbox]')
        ?.getAttribute('aria-labelledby'),
    ).toBe('title')
  })

  it('names the confirm button by its text', () => {
    const { container } = render(
      <Select defaultOpen id="s" type="checkbox">
        <SelectTrigger>Pick</SelectTrigger>
        <SelectContainer confirmButtonText="Done" showConfirmButton>
          <SelectOption value="a">Apple</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const confirm = container.querySelector('[role=listbox] button')!

    expect(confirm).not.toHaveAttribute('aria-label')
    expect(confirm.textContent).toBe('Done')
    fireEvent.click(confirm)
    expect(container.querySelector('[role=listbox]')).toBeNull()
  })
})
