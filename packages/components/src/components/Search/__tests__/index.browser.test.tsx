import { describe, expect, it, mock } from 'bun:test'
import { fireEvent, render, userEvent } from 'bun-test-env-dom'
import { createRef, useState } from 'react'

import { Search } from '../index'

describe('Search', () => {
  it('renders a search input with a magnifier icon', () => {
    const { container } = render(<Search placeholder="Search" />)

    expect(container).toMatchSnapshot()
    const input = container.querySelector('input')!
    expect(input).toHaveAttribute('type', 'search')
    expect(input).toHaveAttribute('enterkeyhint', 'search')
    expect(container.querySelector('[aria-hidden] svg')).toBeInTheDocument()
  })

  it('is a searchbox named by its label', () => {
    const { getByRole } = render(<Search aria-label="Search docs" />)

    expect(getByRole('searchbox', { name: 'Search docs' })).toBeInTheDocument()
  })

  it('takes another icon, or none', () => {
    const custom = render(<Search icon={<span data-testid="icon" />} />)
    expect(custom.getByTestId('icon')).toBeInTheDocument()

    const none = render(<Search icon={null} />)
    expect(none.container.querySelector('[aria-hidden]')).toBeNull()
    expect(none.container.querySelector('input')).toHaveClass(
      'padding-left-0-12px--1',
    )
  })

  it('keeps its own class beside the one given', () => {
    const { container } = render(
      <Search className="mine" classNames={{ input: 'inner' }} />,
    )
    const input = container.querySelector('input')!

    expect(input).toHaveClass('mine')
    expect(input).toHaveClass('inner')
    expect(input.classList.length).toBeGreaterThan(2)
  })

  it('forwards its ref to the input', () => {
    const ref = createRef<HTMLInputElement>()
    render(<Search ref={ref} />)

    expect(ref.current?.type).toBe('search')
  })

  it('searches for the typed query on Enter', async () => {
    const onSearch = mock()
    const onKeyDown = mock()
    const { container } = render(
      <Search onKeyDown={onKeyDown} onSearch={onSearch} />,
    )
    const input = container.querySelector('input')!

    await userEvent.type(input, 'devup{Enter}')

    expect(onSearch).toHaveBeenCalledTimes(1)
    expect(onSearch).toHaveBeenCalledWith('devup')
    expect(onKeyDown).toHaveBeenCalled()
  })

  it('searches for an empty query too', () => {
    const onSearch = mock()
    const { container } = render(<Search onSearch={onSearch} />)

    fireEvent.keyDown(container.querySelector('input')!, { key: 'Enter' })

    expect(onSearch).toHaveBeenCalledWith('')
  })

  it('does not search on the Enter that ends an IME composition', () => {
    const onSearch = mock()
    const { container } = render(
      <Search defaultValue="한글" onSearch={onSearch} />,
    )

    fireEvent.keyDown(container.querySelector('input')!, {
      key: 'Enter',
      isComposing: true,
    })

    expect(onSearch).not.toHaveBeenCalled()
  })

  it('leaves a key its owner handled alone', () => {
    const onSearch = mock()
    const onClear = mock()
    const { container } = render(
      <Search
        defaultValue="query"
        onClear={onClear}
        onKeyDown={(e) => e.preventDefault()}
        onSearch={onSearch}
      />,
    )
    const input = container.querySelector('input')!

    fireEvent.keyDown(input, { key: 'Enter' })
    fireEvent.keyDown(input, { key: 'Escape' })

    expect(onSearch).not.toHaveBeenCalled()
    expect(onClear).not.toHaveBeenCalled()
    expect(input.value).toBe('query')
  })

  it('does nothing on other keys', () => {
    const onSearch = mock()
    const { container } = render(<Search onSearch={onSearch} />)

    fireEvent.keyDown(container.querySelector('input')!, { key: 'a' })

    expect(onSearch).not.toHaveBeenCalled()
  })

  it('clears the query on Escape with a change event', async () => {
    const onChange = mock()
    const onClear = mock()
    const { container } = render(
      <Search defaultValue="query" onChange={onChange} onClear={onClear} />,
    )
    const input = container.querySelector('input')!
    input.focus()

    await userEvent.keyboard('{Escape}')

    expect(input.value).toBe('')
    expect(onClear).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange.mock.calls[0][0].target.value).toBe('')
    expect(container.querySelector('button')).toBeNull()
  })

  it('clears a controlled query on Escape through its owner', async () => {
    function Controlled() {
      const [value, setValue] = useState('query')
      return (
        <>
          <Search onChange={(e) => setValue(e.target.value)} value={value} />
          <output>{value}</output>
        </>
      )
    }
    const { container } = render(<Controlled />)
    container.querySelector('input')!.focus()

    await userEvent.keyboard('{Escape}')

    expect(container.querySelector('output')?.textContent).toBe('')
  })

  it('lets Escape pass when there is nothing to clear', () => {
    const onClear = mock()
    const { container } = render(<Search onClear={onClear} />)

    const notCancelled = fireEvent.keyDown(container.querySelector('input')!, {
      key: 'Escape',
    })

    expect(notCancelled).toBe(true)
    expect(onClear).not.toHaveBeenCalled()
  })

  it.each([
    ['clearing is off', { allowClear: false }],
    ['it is read only', { readOnly: true }],
    ['it is disabled', { disabled: true }],
  ])('keeps the query on Escape when %s', (_, props) => {
    const onClear = mock()
    const { container } = render(
      <Search defaultValue="query" onClear={onClear} {...props} />,
    )
    const input = container.querySelector('input')!

    const notCancelled = fireEvent.keyDown(input, { key: 'Escape' })

    expect(notCancelled).toBe(true)
    expect(input.value).toBe('query')
    expect(onClear).not.toHaveBeenCalled()
  })

  it('clears with the clear button as Input does', async () => {
    const onClear = mock()
    const { container } = render(
      <Search defaultValue="query" onClear={onClear} />,
    )

    await userEvent.click(container.querySelector('button')!)

    expect(container.querySelector('input')!.value).toBe('')
    expect(onClear).toHaveBeenCalledTimes(1)
  })
})
