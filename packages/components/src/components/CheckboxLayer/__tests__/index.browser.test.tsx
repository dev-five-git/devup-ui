import { fireEvent, render, screen } from '@testing-library/react'

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
  it('should render', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render all checkboxes', () => {
    render(<CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />)

    expect(screen.getByText('Option 1')).toBeInTheDocument()
    expect(screen.getByText('Option 2')).toBeInTheDocument()
    expect(screen.getByText('Option 3')).toBeInTheDocument()
  })

  it('should render with row direction', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="row" />,
    )

    const flexContainer = container.firstChild
    expect(flexContainer).toBeInTheDocument()
  })

  it('should render with column direction', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )

    const flexContainer = container.firstChild
    expect(flexContainer).toBeInTheDocument()
  })

  it('should handle checkbox change event', () => {
    const handleCheckboxChange = vi.fn()
    render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={handleCheckboxChange}
      />,
    )

    const firstCheckbox = screen.getByRole('checkbox', { name: /Option 1/i })
    fireEvent.click(firstCheckbox)

    expect(handleCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox1',
      value: 'Option 1',
      checked: true,
      checkedValues: ['Option 1'],
    })
  })

  it('should handle multiple checkbox selections', () => {
    const handleCheckboxChange = vi.fn()
    render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={handleCheckboxChange}
      />,
    )

    const firstCheckbox = screen.getByRole('checkbox', { name: /Option 1/i })
    const secondCheckbox = screen.getByRole('checkbox', { name: /Option 2/i })

    fireEvent.click(firstCheckbox)
    fireEvent.click(secondCheckbox)

    expect(handleCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox2',
      value: 'Option 2',
      checked: true,
      checkedValues: ['Option 1', 'Option 2'],
    })
  })

  it('should handle checkbox unchecking', () => {
    const handleCheckboxChange = vi.fn()
    render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        defaultCheckedIds={['checkbox1']}
        flexDir="column"
        onCheckboxChange={handleCheckboxChange}
      />,
    )

    const firstCheckbox = screen.getByRole('checkbox', { name: /Option 1/i })
    fireEvent.click(firstCheckbox) // uncheck

    expect(handleCheckboxChange).toHaveBeenCalledWith({
      id: 'checkbox1',
      value: 'Option 1',
      checked: false,
      checkedValues: [],
    })
  })

  it('should render with default checked items', () => {
    render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        defaultCheckedIds={['checkbox1', 'checkbox2']}
        flexDir="column"
      />,
    )

    const firstCheckbox = screen.getByRole('checkbox', { name: /Option 1/i })
    const secondCheckbox = screen.getByRole('checkbox', { name: /Option 2/i })
    const thirdCheckbox = screen.getByRole('checkbox', { name: /Option 3/i })

    expect(firstCheckbox).toBeChecked()
    expect(secondCheckbox).toBeChecked()
    expect(thirdCheckbox).not.toBeChecked()
  })

  it('should handle disabled checkboxes', () => {
    render(<CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />)

    const disabledCheckbox = screen.getByRole('checkbox', { name: /Option 3/i })
    expect(disabledCheckbox).toBeDisabled()
  })

  it('should not trigger onChange for disabled checkboxes', () => {
    const handleCheckboxChange = vi.fn()
    render(
      <CheckboxLayer
        checkboxes={mockCheckboxes}
        flexDir="column"
        onCheckboxChange={handleCheckboxChange}
      />,
    )

    const disabledCheckbox = screen.getByRole('checkbox', { name: /Option 3/i })
    fireEvent.click(disabledCheckbox)

    expect(handleCheckboxChange).not.toHaveBeenCalled()
  })

  it('should render all checkboxes correctly', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />,
    )

    const checkboxes = container.querySelectorAll('input[type="checkbox"]')
    expect(checkboxes).toHaveLength(3)
  })

  it('should render with custom gap', () => {
    const { container } = render(
      <CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" gap={20} />,
    )

    const flexContainer = container.firstChild
    expect(flexContainer).toBeInTheDocument()
  })

  it('should generate unique labels for each checkbox', () => {
    render(<CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />)

    const checkbox1 = screen.getByRole('checkbox', { name: /Option 1/i })
    const checkbox2 = screen.getByRole('checkbox', { name: /Option 2/i })
    const checkbox3 = screen.getByRole('checkbox', { name: /Option 3/i })

    expect(checkbox1).toHaveAttribute('id', 'checkbox1-option-1')
    expect(checkbox2).toHaveAttribute('id', 'checkbox2-option-2')
    expect(checkbox3).toHaveAttribute('id', 'checkbox3-option-3')
  })

  it('should handle complex checkbox values', () => {
    const complexCheckboxes = [
      {
        id: 'complex1',
        value: <span data-testid="complex-value">Complex Value</span>,
        label: 'complex-option',
      },
    ]

    const handleCheckboxChange = vi.fn()
    render(
      <CheckboxLayer
        checkboxes={complexCheckboxes}
        flexDir="column"
        onCheckboxChange={handleCheckboxChange}
      />,
    )

    expect(screen.getByTestId('complex-value')).toBeInTheDocument()

    const checkbox = screen.getByRole('checkbox')
    fireEvent.click(checkbox)

    expect(handleCheckboxChange).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'complex1',
        checked: true,
      }),
    )
  })
})
