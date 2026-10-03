import { describe, expect, it, mock } from 'bun:test'
import { act, fireEvent, render } from 'bun-test-env-dom'
import { createRef, type MouseEvent } from 'react'

import { Select, SelectContainer, SelectOption, SelectTrigger } from '../index'

function Menu(props: Partial<React.ComponentProps<typeof Select>>) {
  return (
    <Select {...props}>
      <SelectTrigger>Pick</SelectTrigger>
      <SelectContainer>
        <SelectOption value="a">A</SelectOption>
        <SelectOption value="b">B</SelectOption>
      </SelectContainer>
    </Select>
  )
}

describe('Select open state', () => {
  it('tells the owner of a controlled select when a click outside closes it, and stays as the owner leaves it', () => {
    const onOpenChange = mock()
    const { container } = render(<Menu onOpenChange={onOpenChange} open />)

    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()
    fireEvent.click(document.body)

    expect(onOpenChange).toHaveBeenCalledTimes(1)
    expect(onOpenChange).toHaveBeenCalledWith(false)
    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()
  })

  it('closes an uncontrolled select on a click outside and tells the owner', () => {
    const onOpenChange = mock()
    const { container } = render(
      <Menu defaultOpen onOpenChange={onOpenChange} />,
    )

    fireEvent.click(document.body)

    expect(onOpenChange).toHaveBeenCalledWith(false)
    expect(container.querySelector('[role=listbox]')).not.toBeInTheDocument()
  })

  it('ignores clicks inside the select', () => {
    const onOpenChange = mock()
    const { container } = render(<Menu onOpenChange={onOpenChange} open />)

    fireEvent.click(container.querySelector('[role=listbox]')!)

    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('follows the owner of a controlled select as it opens and closes', () => {
    const { container, rerender } = render(<Menu open={false} />)
    expect(container.querySelector('[role=listbox]')).not.toBeInTheDocument()

    fireEvent.click(container.querySelector('[aria-haspopup=listbox]')!)
    expect(container.querySelector('[role=listbox]')).not.toBeInTheDocument()

    rerender(<Menu open />)
    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()
  })
})

describe('SelectContainer position', () => {
  function open(rect: Partial<DOMRect>, size: { w: number; h: number }) {
    const view = render(<Menu />)
    const root = view.container.firstElementChild as HTMLElement
    root.getBoundingClientRect = () =>
      ({
        x: rect.left ?? 0,
        y: rect.top ?? 0,
        top: rect.top ?? 0,
        left: rect.left ?? 0,
        height: rect.height ?? 40,
        width: rect.width ?? 100,
      }) as DOMRect
    Object.defineProperty(root, 'offsetWidth', { value: rect.width ?? 100 })
    Object.defineProperty(HTMLElement.prototype, 'offsetHeight', {
      configurable: true,
      value: size.h,
    })
    Object.defineProperty(HTMLElement.prototype, 'offsetWidth', {
      configurable: true,
      value: size.w,
    })
    fireEvent.click(view.container.querySelector('[aria-haspopup=listbox]')!)
    return view.container.querySelector<HTMLElement>('[role=listbox]')!
  }

  it('opens below the trigger when it fits the viewport, and keeps no stale bottom', () => {
    const listbox = open({ top: 100, left: 20 }, { w: 200, h: 100 })
    expect(listbox.style.top).toBe('150px')
    expect(listbox.style.bottom).toBe('')
    expect(listbox.style.left).toBe('20px')
  })

  it('opens above the trigger when it would leave the bottom of the viewport, and keeps no stale top', () => {
    const listbox = open(
      { top: window.innerHeight - 60, left: 20 },
      { w: 200, h: 300 },
    )
    expect(listbox.style.top).toBe('')
    expect(listbox.style.bottom).toBe('70px')
  })

  it('aligns to the right edge of the trigger when it would leave the right of the viewport', () => {
    const listbox = open(
      { top: 10, left: window.innerWidth - 50, width: 50 },
      { w: 200, h: 100 },
    )
    expect(listbox.style.left).toBe(`${window.innerWidth - 200}px`)
  })

  it('moves between the two sides as the window resizes', () => {
    const listbox = open({ top: 100, left: 20 }, { w: 200, h: 100 })
    expect(listbox.style.top).toBe('150px')

    const original = window.innerHeight
    window.innerHeight = 180
    act(() => {
      window.dispatchEvent(new Event('resize'))
    })

    expect(listbox.style.top).toBe('')
    expect(listbox.style.bottom).toBe('90px')
    window.innerHeight = original
  })
})

describe('SelectTrigger asChild', () => {
  it('opens the select as the child handles its own click, and honors a prevented event', () => {
    const onClick = mock((e: MouseEvent<HTMLElement>) => e.type)
    const { container } = render(
      <Select>
        <SelectTrigger asChild>
          <button data-testid="child" onClick={onClick} type="button">
            Open
          </button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">A</SelectOption>
        </SelectContainer>
      </Select>,
    )

    fireEvent.click(container.querySelector('button')!)
    expect(onClick).toHaveBeenCalledTimes(1)
    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()

    fireEvent.click(container.querySelector('button')!)
    expect(container.querySelector('[role=listbox]')).not.toBeInTheDocument()

    const prevented = render(
      <Select>
        <SelectTrigger asChild>
          <button onClick={(e) => e.preventDefault()} type="button">
            Open
          </button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">A</SelectOption>
        </SelectContainer>
      </Select>,
    )
    fireEvent.click(prevented.container.querySelector('button')!)
    expect(
      prevented.container.querySelector('[role=listbox]'),
    ).not.toBeInTheDocument()
  })

  it('composes the key handler of the child and the one given to the trigger', () => {
    const childKey = mock()
    const triggerKey = mock()
    const { container } = render(
      <Select>
        <SelectTrigger asChild onKeyDown={triggerKey}>
          <button onKeyDown={childKey} type="button">
            Open
          </button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">A</SelectOption>
        </SelectContainer>
      </Select>,
    )

    fireEvent.keyDown(container.querySelector('button')!, { key: 'ArrowDown' })

    expect(childKey).toHaveBeenCalledTimes(1)
    expect(triggerKey).toHaveBeenCalledTimes(1)
    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()
  })

  it('keeps the state attributes of the trigger and the label of the child', () => {
    const { container } = render(
      <Select>
        <SelectTrigger asChild className="from-trigger" id="trigger">
          <button
            aria-expanded="nope"
            aria-label="Choose"
            className="from-child"
            type="button"
          >
            Open
          </button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">A</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const button = container.querySelector('button')!

    expect(button).toHaveAttribute('aria-label', 'Choose')
    expect(button).toHaveAttribute('aria-expanded', 'false')
    expect(button).toHaveAttribute('aria-haspopup', 'listbox')
    expect(button).toHaveAttribute('id', 'trigger')
    expect(button.className.split(' ')).toEqual(
      expect.arrayContaining(['from-trigger', 'from-child']),
    )
    expect(button.getAttribute('aria-controls')).toMatch(/-listbox$/)
  })

  it('hands the element to the ref of the child and the ref of the trigger', () => {
    const childRef = createRef<HTMLButtonElement>()
    const triggerRef = createRef<HTMLElement>()
    const { container, unmount } = render(
      <Select>
        <SelectTrigger ref={triggerRef} asChild>
          <button ref={childRef} type="button">
            Open
          </button>
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">A</SelectOption>
        </SelectContainer>
      </Select>,
    )
    const button = container.querySelector('button')

    expect(childRef.current).toBe(button)
    expect(triggerRef.current).toBe(button)
    unmount()
    expect(childRef.current).toBeNull()
    expect(triggerRef.current).toBeNull()
  })

  it('hands the button to the ref of the trigger', () => {
    const triggerRef = createRef<HTMLElement>()
    const onClick = mock()
    const { container } = render(
      <Select>
        <SelectTrigger ref={triggerRef} onClick={onClick}>
          Open
        </SelectTrigger>
        <SelectContainer>
          <SelectOption value="a">A</SelectOption>
        </SelectContainer>
      </Select>,
    )

    expect(triggerRef.current).toBe(container.querySelector('button'))
    fireEvent.click(container.querySelector('button')!)
    expect(onClick).toHaveBeenCalledTimes(1)
    expect(container.querySelector('[role=listbox]')).toBeInTheDocument()
  })

  it('refuses a child that is not an element', () => {
    expect(() =>
      render(
        <Select>
          <SelectTrigger asChild>text</SelectTrigger>
        </Select>,
      ),
    ).toThrow('SelectTrigger with asChild requires a single element')
  })
})
