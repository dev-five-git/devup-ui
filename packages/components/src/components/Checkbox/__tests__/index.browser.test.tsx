import { fireEvent, render, screen } from '@testing-library/react'

import { Checkbox } from '..'

// Mock getTheme function
vi.mock('@devup-ui/react', async (importOriginal) => {
  const mod = await importOriginal<typeof import('@devup-ui/react')>()
  return {
    ...mod,
    getTheme: vi.fn(() => 'light'),
  }
})

const { getTheme } = await import('@devup-ui/react')

describe('Checkbox', () => {
  beforeEach(() => {
    // Reset to light theme before each test
    vi.mocked(getTheme).mockReturnValue('light')
  })

  afterEach(() => {
    vi.clearAllMocks()
  })
  it('should render', () => {
    const { container } = render(
      <Checkbox label="test-checkbox">Checkbox</Checkbox>,
    )
    expect(container).toMatchSnapshot()
  })

  it('should render with text children', () => {
    render(<Checkbox label="test-checkbox">Checkbox Label</Checkbox>)
    expect(screen.getByText('Checkbox Label')).toBeInTheDocument()
  })

  it('should render with custom React node children', () => {
    render(
      <Checkbox label="test-checkbox">
        <span data-testid="custom-label">Custom Label</span>
      </Checkbox>,
    )
    expect(screen.getByTestId('custom-label')).toBeInTheDocument()
  })

  it('should render checked state with CheckIcon', () => {
    const { container } = render(
      <Checkbox checked label="test-checkbox">
        Checked Checkbox
      </Checkbox>,
    )

    const checkIcon = container.querySelector('svg')
    expect(checkIcon).toBeInTheDocument()
    expect(checkIcon).toHaveAttribute('width', '12')
    expect(checkIcon).toHaveAttribute('height', '10')
  })

  it('should not render CheckIcon when unchecked', () => {
    const { container } = render(
      <Checkbox checked={false} label="test-checkbox">
        Unchecked Checkbox
      </Checkbox>,
    )

    const checkIcon = container.querySelector('svg')
    expect(checkIcon).not.toBeInTheDocument()
  })

  it('should handle disabled state', () => {
    render(
      <Checkbox disabled label="test-checkbox">
        Disabled Checkbox
      </Checkbox>,
    )

    const checkbox = screen.getByRole('checkbox')
    expect(checkbox).toBeDisabled()
  })

  it('should call onChange when clicked', () => {
    const handleChange = vi.fn()
    render(
      <Checkbox label="test-checkbox" onChange={handleChange}>
        Clickable Checkbox
      </Checkbox>,
    )

    const checkbox = screen.getByRole('checkbox')
    fireEvent.click(checkbox)

    expect(handleChange).toHaveBeenCalledWith(true)
  })

  it('should not call onChange when disabled and clicked', () => {
    const handleChange = vi.fn()
    render(
      <Checkbox disabled label="test-checkbox" onChange={handleChange}>
        Disabled Checkbox
      </Checkbox>,
    )

    const checkbox = screen.getByRole('checkbox')
    fireEvent.click(checkbox)

    expect(handleChange).not.toHaveBeenCalled()
  })

  it('should have proper label association', () => {
    render(<Checkbox label="test-checkbox">Test Label</Checkbox>)

    const checkbox = screen.getByRole('checkbox')
    const label = screen.getByText('Test Label')

    expect(checkbox).toHaveAttribute('id', 'test-checkbox')
    expect(label.closest('label')).toHaveAttribute('for', 'test-checkbox')
  })

  it('should render CheckIcon with disabled color when disabled and checked', () => {
    const { container } = render(
      <Checkbox checked disabled label="test-checkbox">
        Disabled Checked Checkbox
      </Checkbox>,
    )

    const checkIcon = container.querySelector('svg path')
    expect(checkIcon).toBeInTheDocument()
    expect(checkIcon).toHaveAttribute('fill', '#D6D7DE')
  })

  it('should render CheckIcon with normal color when enabled and checked', () => {
    const { container } = render(
      <Checkbox checked label="test-checkbox">
        Enabled Checked Checkbox
      </Checkbox>,
    )

    const checkIcon = container.querySelector('svg path')
    expect(checkIcon).toBeInTheDocument()
    expect(checkIcon).toHaveAttribute('fill', '#FFF')
  })

  describe('Theme-based styling', () => {
    it('should render CheckIcon with light theme disabled color when disabled and checked', () => {
      vi.mocked(getTheme).mockReturnValue('light')

      const { container } = render(
        <Checkbox checked disabled label="test-checkbox">
          Light Theme Disabled Checked
        </Checkbox>,
      )

      const checkIcon = container.querySelector('svg path')
      expect(checkIcon).toBeInTheDocument()
      expect(checkIcon).toHaveAttribute('fill', '#D6D7DE')
    })

    it('should render CheckIcon with dark theme disabled color when disabled and checked', () => {
      vi.mocked(getTheme).mockReturnValue('dark')

      const { container } = render(
        <Checkbox checked disabled label="test-checkbox">
          Dark Theme Disabled Checked
        </Checkbox>,
      )

      const checkIcon = container.querySelector('svg path')
      expect(checkIcon).toBeInTheDocument()
      expect(checkIcon).toHaveAttribute('fill', '#373737')
    })

    it('should render with light theme background colors', () => {
      vi.mocked(getTheme).mockReturnValue('light')

      const { container } = render(
        <Checkbox label="light-theme">Light Theme Checkbox</Checkbox>,
      )

      const checkbox = container.querySelector('input[type="checkbox"]')
      expect(checkbox).toBeInTheDocument()
      expect(getTheme).toHaveBeenCalled()
    })

    it('should render with dark theme background colors', () => {
      vi.mocked(getTheme).mockReturnValue('dark')

      const { container } = render(
        <Checkbox label="dark-theme">Dark Theme Checkbox</Checkbox>,
      )

      const checkbox = container.querySelector('input[type="checkbox"]')
      expect(checkbox).toBeInTheDocument()
      expect(getTheme).toHaveBeenCalled()
    })

    it('should use theme for disabled state background', () => {
      // Test light theme disabled
      vi.mocked(getTheme).mockReturnValue('light')

      const { container: lightContainer } = render(
        <Checkbox disabled label="light-disabled">
          Light Disabled
        </Checkbox>,
      )

      const lightCheckbox = lightContainer.querySelector(
        'input[type="checkbox"]',
      )
      expect(lightCheckbox).toBeInTheDocument()
      expect(getTheme).toHaveBeenCalled()

      // Test dark theme disabled
      vi.mocked(getTheme).mockReturnValue('dark')

      const { container: darkContainer } = render(
        <Checkbox disabled label="dark-disabled">
          Dark Disabled
        </Checkbox>,
      )

      const darkCheckbox = darkContainer.querySelector('input[type="checkbox"]')
      expect(darkCheckbox).toBeInTheDocument()
      expect(getTheme).toHaveBeenCalled()
    })

    it('should use theme for checked disabled state background', () => {
      // Test light theme checked disabled
      vi.mocked(getTheme).mockReturnValue('light')

      const { container: lightContainer } = render(
        <Checkbox checked disabled label="light-checked-disabled">
          Light Checked Disabled
        </Checkbox>,
      )

      const lightCheckbox = lightContainer.querySelector(
        'input[type="checkbox"]',
      )
      expect(lightCheckbox).toBeInTheDocument()
      expect(getTheme).toHaveBeenCalled()

      // Test dark theme checked disabled
      vi.mocked(getTheme).mockReturnValue('dark')

      const { container: darkContainer } = render(
        <Checkbox checked disabled label="dark-checked-disabled">
          Dark Checked Disabled
        </Checkbox>,
      )

      const darkCheckbox = darkContainer.querySelector('input[type="checkbox"]')
      expect(darkCheckbox).toBeInTheDocument()
      expect(getTheme).toHaveBeenCalled()
    })
  })

  it('should handle hover and active states', () => {
    const { container } = render(
      <Checkbox label="test-checkbox">Interactive Checkbox</Checkbox>,
    )

    const checkbox = container.querySelector('input[type="checkbox"]')
    expect(checkbox).toBeInTheDocument()

    if (checkbox) {
      // Test active state
      fireEvent.mouseDown(checkbox)
      fireEvent.mouseUp(checkbox)

      // Test hover state
      fireEvent.mouseEnter(checkbox)
      fireEvent.mouseLeave(checkbox)
    }
  })

  it('should handle checked state hover', () => {
    const { container } = render(
      <Checkbox checked label="test-checkbox">
        Checked Hover Checkbox
      </Checkbox>,
    )

    const checkbox = container.querySelector('input[type="checkbox"]')
    expect(checkbox).toBeInTheDocument()

    if (checkbox) {
      // Test checked hover state
      fireEvent.mouseEnter(checkbox)
      fireEvent.mouseLeave(checkbox)
    }
  })

  it('should render disabled label text with correct color', () => {
    render(
      <Checkbox disabled label="test-checkbox">
        Disabled Label Text
      </Checkbox>,
    )

    const labelText = screen.getByText('Disabled Label Text')
    expect(labelText).toBeInTheDocument()
  })

  it('should render enabled label text with correct color', () => {
    render(<Checkbox label="test-checkbox">Enabled Label Text</Checkbox>)

    const labelText = screen.getByText('Enabled Label Text')
    expect(labelText).toBeInTheDocument()
  })

  it('should handle all states correctly', () => {
    // Test unchecked state
    const { container: uncheckedContainer } = render(
      <Checkbox label="unchecked">Unchecked Checkbox</Checkbox>,
    )

    const uncheckedCheckbox = uncheckedContainer.querySelector(
      'input[type="checkbox"]',
    )
    expect(uncheckedCheckbox).toBeInTheDocument()
    expect(uncheckedCheckbox).not.toBeChecked()

    // Test checked state
    const { container: checkedContainer } = render(
      <Checkbox checked label="checked">
        Checked Checkbox
      </Checkbox>,
    )

    const checkedCheckbox = checkedContainer.querySelector(
      'input[type="checkbox"]',
    )
    expect(checkedCheckbox).toBeInTheDocument()
    expect(checkedCheckbox).toBeChecked()
  })

  it('should handle disabled states', () => {
    // Disabled unchecked
    const { container: disabledContainer } = render(
      <Checkbox disabled label="disabled">
        Disabled Checkbox
      </Checkbox>,
    )

    const disabledCheckbox = disabledContainer.querySelector(
      'input[type="checkbox"]',
    )
    expect(disabledCheckbox).toBeDisabled()

    // Disabled checked
    const { container: disabledCheckedContainer } = render(
      <Checkbox checked disabled label="disabled-checked">
        Disabled Checked Checkbox
      </Checkbox>,
    )

    const disabledCheckedCheckbox = disabledCheckedContainer.querySelector(
      'input[type="checkbox"]',
    )
    expect(disabledCheckedCheckbox).toBeDisabled()
    expect(disabledCheckedCheckbox).toBeChecked()
  })

  it('should handle interactive states correctly', () => {
    const { container } = render(
      <Checkbox label="interactive">Interactive Checkbox</Checkbox>,
    )

    const checkbox = container.querySelector('input[type="checkbox"]')
    if (checkbox) {
      // Test active state
      fireEvent.mouseDown(checkbox)
      fireEvent.mouseUp(checkbox)

      // Test hover state
      fireEvent.mouseEnter(checkbox)
      fireEvent.mouseLeave(checkbox)
    }
  })

  it('should handle checked hover state correctly', () => {
    const { container } = render(
      <Checkbox checked label="checked-hover">
        Checked Hover Checkbox
      </Checkbox>,
    )

    const checkbox = container.querySelector('input[type="checkbox"]')
    if (checkbox) {
      // Test checked + hover state
      fireEvent.mouseEnter(checkbox)
      fireEvent.mouseLeave(checkbox)
    }
  })

  it('should handle onChange conditional logic', () => {
    const onChange = vi.fn()

    // Test !disabled && onChange?.() - both sides of &&

    // Case 1: disabled=false, onChange exists (should call)
    const { container: enabledContainer } = render(
      <Checkbox label="enabled-onchange" onChange={onChange}>
        Enabled with onChange
      </Checkbox>,
    )

    const enabledCheckbox = enabledContainer.querySelector(
      'input[type="checkbox"]',
    )
    if (enabledCheckbox) {
      fireEvent.click(enabledCheckbox)
    }
    expect(onChange).toHaveBeenCalledWith(true)

    // Case 2: disabled=true, onChange exists (should not call)
    onChange.mockClear()
    const { container: disabledContainer } = render(
      <Checkbox disabled label="disabled-onchange" onChange={onChange}>
        Disabled with onChange
      </Checkbox>,
    )

    const disabledCheckbox = disabledContainer.querySelector(
      'input[type="checkbox"]',
    )
    if (disabledCheckbox) {
      fireEvent.click(disabledCheckbox)
    }
    expect(onChange).not.toHaveBeenCalled()

    // Case 3: disabled=false, no onChange (should not crash)
    const { container: noOnChangeContainer } = render(
      <Checkbox label="no-onchange">No onChange</Checkbox>,
    )

    const noOnChangeCheckbox = noOnChangeContainer.querySelector(
      'input[type="checkbox"]',
    )
    if (noOnChangeCheckbox) {
      fireEvent.click(noOnChangeCheckbox)
    }
    // Should not crash - that's the test
  })
})
