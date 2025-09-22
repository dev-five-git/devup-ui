import { act, render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { CheckboxLayer } from '..'

const mockCheckboxes = [
  {
    id: 'checkbox1',
    value: 'Option 1',
    label: 'option-1',
  },
  {
    id: 'checkbox2',
    value: 'Option 2',
    label: 'option-2',
  },
  {
    id: 'checkbox3',
    value: 'Option 3',
    label: 'option-3',
    disabled: true,
  },
]

describe('CheckboxLayer', () => {
  it('should render with column layout', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render with row layout', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="row" />,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render with custom gap', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" gap={20} />,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render all checkboxes from the provided array', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )

    const checkboxes = container.querySelectorAll('input[type="checkbox"]')
    expect(checkboxes).toHaveLength(3)

    // Check if all checkboxes have correct IDs and labels
    expect(container.querySelector('#checkbox1-option-1')).toBeInTheDocument()
    expect(container.querySelector('#checkbox2-option-2')).toBeInTheDocument()
    expect(container.querySelector('#checkbox3-option-3')).toBeInTheDocument()
  })

  it('should render disabled checkbox correctly', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )

    const disabledCheckbox = container.querySelector('#checkbox3-option-3')
    expect(disabledCheckbox).toBeDisabled()
  })

  it('should handle defaultCheckedIds', () => {
    const { container } = render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        defaultCheckedIds={['checkbox1', 'checkbox2']}
        flexDir="column"
      />,
    )

    const checkbox1 = container.querySelector('#checkbox1-option-1')
    const checkbox2 = container.querySelector('#checkbox2-option-2')
    const checkbox3 = container.querySelector('#checkbox3-option-3')

    expect(checkbox1).toBeChecked()
    expect(checkbox2).toBeChecked()
    expect(checkbox3).not.toBeChecked()
  })

  it('should call onCheckboxChange when checkbox is clicked', async () => {
    const onCheckboxChange = vi.fn()
    const { container } = render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={onCheckboxChange}
      />,
    )

    const checkbox1 = container.querySelector('#checkbox1-option-1')
    expect(checkbox1).toBeInTheDocument()

    await act(async () => {
      await userEvent.click(checkbox1!)
    })

    expect(onCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox1',
      value: 'Option 1',
      checked: true,
      checkedValues: ['Option 1'],
    })
  })

  it('should handle multiple checkbox selections', async () => {
    const onCheckboxChange = vi.fn()
    const { container } = render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={onCheckboxChange}
      />,
    )

    const checkbox1 = container.querySelector('#checkbox1-option-1')
    const checkbox2 = container.querySelector('#checkbox2-option-2')

    // Check first checkbox
    await act(async () => {
      await userEvent.click(checkbox1!)
    })

    expect(onCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox1',
      value: 'Option 1',
      checked: true,
      checkedValues: ['Option 1'],
    })

    // Check second checkbox
    await act(async () => {
      await userEvent.click(checkbox2!)
    })

    expect(onCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox2',
      value: 'Option 2',
      checked: true,
      checkedValues: ['Option 1', 'Option 2'],
    })
  })

  it('should handle checkbox unchecking', async () => {
    const onCheckboxChange = vi.fn()
    const { container } = render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        defaultCheckedIds={['checkbox1', 'checkbox2']}
        flexDir="column"
        onCheckboxChange={onCheckboxChange}
      />,
    )

    const checkbox1 = container.querySelector('#checkbox1-option-1')

    // Uncheck first checkbox
    await act(async () => {
      await userEvent.click(checkbox1!)
    })

    expect(onCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox1',
      value: 'Option 1',
      checked: false,
      checkedValues: ['Option 2'],
    })
  })

  it('should not call onCheckboxChange for disabled checkbox', async () => {
    const onCheckboxChange = vi.fn()
    const { container } = render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={onCheckboxChange}
      />,
    )

    const disabledCheckbox = container.querySelector('#checkbox3-option-3')

    await act(async () => {
      await userEvent.click(disabledCheckbox!)
    })

    expect(onCheckboxChange).not.toHaveBeenCalled()
  })

  it('should work without onCheckboxChange callback', async () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )

    const checkbox1 = container.querySelector('#checkbox1-option-1')

    // Should not throw error when clicking without callback
    await act(async () => {
      await userEvent.click(checkbox1!)
    })

    expect(true).toBe(true) // Test passes if no error is thrown
  })

  it('should handle empty checkboxes array', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={[]} flexDir="column" />,
    )
    expect(container).toMatchSnapshot()

    const checkboxes = container.querySelectorAll('input[type="checkbox"]')
    expect(checkboxes).toHaveLength(0)
  })

  it('should use default gap when not provided', () => {
    const { container: columnContainer } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )
    const { container: rowContainer } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="row" />,
    )

    expect(columnContainer).toMatchSnapshot()
    expect(rowContainer).toMatchSnapshot()
  })

  it('should handle React element as checkbox value', async () => {
    const customCheckboxes = [
      {
        id: 'custom1',
        value: <span>Custom Element</span>,
        label: 'custom-1',
      },
      {
        id: 'custom2',
        value: <div>Another Custom Element</div>,
        label: 'custom-2',
      },
    ]

    const onCheckboxChange = vi.fn()
    const { container } = render(
      <CheckboxLayer
        checkboxes={customCheckboxes}
        flexDir="column"
        onCheckboxChange={onCheckboxChange}
      />,
    )

    const customSpan = container.querySelector('span')
    expect(customSpan).toHaveTextContent('Custom Element')

    const customDiv = container.querySelector('div')
    expect(customDiv).toHaveTextContent('Another Custom Element')

    const checkbox1 = container.querySelector('#custom1-custom-1')
    await act(async () => {
      await userEvent.click(checkbox1!)
    })

    // Check that React element is passed correctly to the callback
    expect(onCheckboxChange).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'custom1',
        checked: true,
        checkedValues: [expect.any(Object)], // React element
      }),
    )
  })

  it('should maintain internal state correctly across multiple interactions', async () => {
    const onCheckboxChange = vi.fn()
    const { container } = render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={onCheckboxChange}
      />,
    )

    const checkbox1 = container.querySelector('#checkbox1-option-1')
    const checkbox2 = container.querySelector('#checkbox2-option-2')

    // Check both checkboxes
    await act(async () => {
      await userEvent.click(checkbox1!)
    })
    await act(async () => {
      await userEvent.click(checkbox2!)
    })

    // Uncheck first checkbox
    await act(async () => {
      await userEvent.click(checkbox1!)
    })

    // The last call should show only checkbox2 in checkedValues
    expect(onCheckboxChange).toHaveBeenLastCalledWith({
      id: 'checkbox1',
      value: 'Option 1',
      checked: false,
      checkedValues: ['Option 2'],
    })

    expect(onCheckboxChange).toHaveBeenCalledTimes(3)
  })
})
