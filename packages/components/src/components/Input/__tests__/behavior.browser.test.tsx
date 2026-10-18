import { describe, expect, it, mock } from 'bun:test'
import { fireEvent, render, userEvent } from 'bun-test-env-dom'
import { createRef, useState } from 'react'

import { Textarea } from '../../Textarea'
import { Input } from '../index'

describe('Input accessible name and description', () => {
  it('takes its name from an associated label, not from a forced label', () => {
    const { container } = render(
      <>
        <label htmlFor="email">Email</label>
        <Input id="email" />
      </>,
    )
    const input = container.querySelector('input')!

    expect(input).not.toHaveAttribute('aria-label')
    expect(input.labels?.[0]?.textContent).toBe('Email')
  })

  it('marks an error and describes the control with its message', () => {
    const { container } = render(
      <Input
        aria-describedby="hint"
        error
        errorMessage="Too short"
        id="name"
      />,
    )
    const input = container.querySelector('input')!
    const message = container.querySelector('[role=alert]')!

    expect(input).toHaveAttribute('aria-invalid', 'true')
    expect(message.textContent).toBe('Too short')
    expect(input.getAttribute('aria-describedby')?.split(' ')).toEqual([
      'hint',
      message.id,
    ])
    expect(message.id).toBe('name-error')
  })

  it('gives an input without an id a stable message id', () => {
    const { container } = render(<Input error errorMessage="Required" />)
    const input = container.querySelector('input')!
    const message = container.querySelector('[role=alert]')!

    expect(message.id).not.toBe('')
    expect(input).toHaveAttribute('aria-describedby', message.id)
  })

  it('claims nothing without an error', () => {
    const { container } = render(<Input errorMessage="Too short" />)
    const input = container.querySelector('input')!

    expect(input).not.toHaveAttribute('aria-invalid')
    expect(input).not.toHaveAttribute('aria-describedby')
    expect(container.querySelector('[role=alert]')).toBeNull()
  })

  it('hides a decorative icon from assistive technology', () => {
    const { container } = render(<Input icon={<svg />} />)

    expect(container.querySelector('[aria-hidden=true]')).toBeInTheDocument()
  })
})

describe('Input clear', () => {
  it('clears with a real change event the owner of a controlled input can use', async () => {
    const seen: { type: string; value: string; target: EventTarget | null }[] =
      []
    const onClear = mock()
    function Controlled() {
      const [value, setValue] = useState('hello')
      return (
        <Input
          name="q"
          onChange={(e) => {
            seen.push({
              type: e.type,
              value: e.currentTarget.value,
              target: e.target,
            })
            setValue(e.target.value)
          }}
          onClear={onClear}
          value={value}
        />
      )
    }
    const { container } = render(<Controlled />)
    const input = container.querySelector('input')!

    await userEvent.click(container.querySelector('button')!)

    expect(input.value).toBe('')
    expect(seen).toEqual([{ type: 'change', value: '', target: input }])
    expect(onClear).toHaveBeenCalledTimes(1)
    expect(document.activeElement).toBe(input)
    expect(container.querySelector('button')).toBeNull()
  })

  it('does not clear a controlled input whose owner keeps the value', async () => {
    const { container } = render(<Input onChange={() => {}} value="kept" />)

    await userEvent.click(container.querySelector('button')!)

    expect(container.querySelector('input')!.value).toBe('kept')
  })

  it('clears an uncontrolled input and tells the owner of the change', async () => {
    const onChange = mock()
    const { container } = render(
      <Input defaultValue="typed" onChange={onChange} />,
    )

    await userEvent.click(container.querySelector('button')!)

    expect(container.querySelector('input')!.value).toBe('')
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange.mock.calls[0][0].target.value).toBe('')
  })

  it('reports typing as it did', () => {
    const onChange = mock()
    const { container } = render(<Input onChange={onChange} />)

    fireEvent.change(container.querySelector('input')!, {
      target: { value: 'a' },
    })

    expect(onChange).toHaveBeenCalledTimes(1)
  })

  it('hands the input to both the ref of the caller and the clear button', () => {
    const ref = createRef<HTMLInputElement>()
    const { container } = render(<Input ref={ref} defaultValue="x" />)

    expect(ref.current).toBe(container.querySelector('input'))
  })

  it('still reports the clear of an input that is not mounted yet by the owner', () => {
    const onClear = mock()
    const { container } = render(<Input defaultValue="x" onClear={onClear} />)

    fireEvent.click(container.querySelector('button')!)

    expect(onClear).toHaveBeenCalledTimes(1)
  })
})

describe('Textarea accessible name and description', () => {
  it('takes its name from an associated label, not from a forced label', () => {
    const { container } = render(
      <>
        <label htmlFor="bio">Bio</label>
        <Textarea id="bio" />
      </>,
    )
    const textarea = container.querySelector('textarea')!

    expect(textarea).not.toHaveAttribute('aria-label')
    expect(textarea.labels?.[0]?.textContent).toBe('Bio')
  })

  it('marks an error and describes the control with its message', () => {
    const { container } = render(
      <Textarea
        aria-describedby="hint"
        error
        errorMessage="Too long"
        id="bio"
      />,
    )
    const textarea = container.querySelector('textarea')!
    const message = container.querySelector('[role=alert]')!

    expect(textarea).toHaveAttribute('aria-invalid', 'true')
    expect(textarea.getAttribute('aria-describedby')?.split(' ')).toEqual([
      'hint',
      'bio-error',
    ])
    expect(message.id).toBe('bio-error')
    expect(message.textContent).toBe('Too long')
  })

  it('claims nothing without an error, and gives its own message id otherwise', () => {
    const quiet = render(<Textarea errorMessage="Too long" />)
    expect(quiet.container.querySelector('textarea')).not.toHaveAttribute(
      'aria-describedby',
    )

    const loud = render(<Textarea error errorMessage="Too long" />)
    expect(
      loud.container
        .querySelector('textarea')
        ?.getAttribute('aria-describedby'),
    ).toBe(loud.container.querySelector('[role=alert]')?.id)
  })
})
