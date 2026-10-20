import { describe, expect, it, mock } from 'bun:test'
import { fireEvent, render } from 'bun-test-env-dom'

import { Button } from '../index'

describe('Button accessible name', () => {
  it('takes its name from its text, not from a forced label', () => {
    const { container } = render(<Button>Save changes</Button>)
    const button = container.querySelector('button')!

    expect(button).not.toHaveAttribute('aria-label')
    expect(button.textContent).toBe('Save changes')
  })

  it('keeps a label given by the caller, as an icon-only button needs', () => {
    const { container } = render(<Button aria-label="Close" icon={<svg />} />)

    expect(container.querySelector('button')).toHaveAttribute(
      'aria-label',
      'Close',
    )
  })
})

describe('Button loading', () => {
  it('announces the busy state and cannot be activated twice', () => {
    const onClick = mock()
    const { container } = render(
      <Button loading onClick={onClick}>
        Save
      </Button>,
    )
    const button = container.querySelector('button')!

    expect(button).toHaveAttribute('aria-busy', 'true')
    expect(button).toBeDisabled()
    expect(button).toHaveAttribute('aria-disabled', 'true')
    fireEvent.click(button)
    expect(onClick).not.toHaveBeenCalled()
  })

  it('tells assistive technology what it is waiting for', () => {
    const { container } = render(
      <Button loading loadingLabel="Saving">
        Save
      </Button>,
    )
    const status = container.querySelector('[role=status]')

    expect(status?.textContent).toBe('Saving')
  })

  it('says Loading by default and says nothing when it is not loading', () => {
    const loading = render(<Button loading>Save</Button>)
    expect(loading.container.querySelector('[role=status]')?.textContent).toBe(
      'Loading',
    )

    const idle = render(<Button>Save</Button>)
    expect(idle.container.querySelector('[role=status]')).toBeNull()
    expect(idle.container.querySelector('button')).not.toHaveAttribute(
      'aria-busy',
    )
    expect(idle.container.querySelector('button')).not.toHaveAttribute(
      'aria-disabled',
    )
  })

  it('stays actionable while loading when it is told not to disable', () => {
    const onClick = mock()
    const { container } = render(
      <Button disableWhileLoading={false} loading onClick={onClick}>
        Save
      </Button>,
    )
    const button = container.querySelector('button')!

    expect(button).toHaveAttribute('aria-busy', 'true')
    expect(button).not.toBeDisabled()
    fireEvent.click(button)
    expect(onClick).toHaveBeenCalledTimes(1)
  })

  it('stays disabled when the caller disables it, loading or not', () => {
    const { container } = render(
      <Button disabled disableWhileLoading={false}>
        Save
      </Button>,
    )

    expect(container.querySelector('button')).toBeDisabled()
  })
})
