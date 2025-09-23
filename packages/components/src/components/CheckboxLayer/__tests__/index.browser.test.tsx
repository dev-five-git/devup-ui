import { fireEvent, render, screen } from '@testing-library/react'

import { CheckboxLayer } from '..'

const mockCheckboxes = [
  {
    id: 'checkbox1',
    value: 'Option 1',
  },
  {
    id: 'checkbox2',
    value: 'Option 2',
  },
  {
    id: 'checkbox3',
    value: 'Option 3',
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

  describe('handleCheckboxChange function', () => {
    it('should call onCheckboxChange when checkbox is selected', () => {
      const mockOnCheckboxChange = vi.fn()

      render(
        <CheckboxLayer
          checkboxes={mockCheckboxes}
          flexDir="column"
          onCheckboxChange={mockOnCheckboxChange}
        />,
      )

      const checkbox1 = screen.getByLabelText('Option 1')
      fireEvent.click(checkbox1)

      expect(mockOnCheckboxChange).toHaveBeenCalledWith({
        id: 'checkbox1',
        value: 'Option 1',
        checked: true,
        checkedValues: ['Option 1'],
      })
    })

    it('should call onCheckboxChange when checkbox is deselected', () => {
      const mockOnCheckboxChange = vi.fn()

      render(
        <CheckboxLayer
          checkboxes={mockCheckboxes}
          defaultCheckedIds={['checkbox1']}
          flexDir="column"
          onCheckboxChange={mockOnCheckboxChange}
        />,
      )

      const checkbox1 = screen.getByLabelText('Option 1')
      fireEvent.click(checkbox1)

      expect(mockOnCheckboxChange).toHaveBeenCalledWith({
        id: 'checkbox1',
        value: 'Option 1',
        checked: false,
        checkedValues: [],
      })
    })

    it('should handle multiple checkbox selections correctly', () => {
      const mockOnCheckboxChange = vi.fn()

      render(
        <CheckboxLayer
          checkboxes={mockCheckboxes}
          flexDir="column"
          onCheckboxChange={mockOnCheckboxChange}
        />,
      )

      // 첫 번째 체크박스 선택
      const checkbox1 = screen.getByLabelText('Option 1')
      fireEvent.click(checkbox1)

      expect(mockOnCheckboxChange).toHaveBeenCalledWith({
        id: 'checkbox1',
        value: 'Option 1',
        checked: true,
        checkedValues: ['Option 1'],
      })

      // 두 번째 체크박스 선택
      const checkbox2 = screen.getByLabelText('Option 2')
      fireEvent.click(checkbox2)

      expect(mockOnCheckboxChange).toHaveBeenCalledWith({
        id: 'checkbox2',
        value: 'Option 2',
        checked: true,
        checkedValues: ['Option 1', 'Option 2'],
      })
    })

    it('should maintain correct checkedValues order when deselecting middle item', () => {
      const mockOnCheckboxChange = vi.fn()

      render(
        <CheckboxLayer
          checkboxes={mockCheckboxes}
          defaultCheckedIds={['checkbox1', 'checkbox2']}
          flexDir="column"
          onCheckboxChange={mockOnCheckboxChange}
        />,
      )

      // 첫 번째 체크박스 해제
      const checkbox1 = screen.getByLabelText('Option 1')
      fireEvent.click(checkbox1)

      expect(mockOnCheckboxChange).toHaveBeenCalledWith({
        id: 'checkbox1',
        value: 'Option 1',
        checked: false,
        checkedValues: ['Option 2'],
      })
    })

    it('should work with defaultCheckedIds', () => {
      const mockOnCheckboxChange = vi.fn()

      render(
        <CheckboxLayer
          checkboxes={mockCheckboxes}
          defaultCheckedIds={['checkbox1', 'checkbox2']}
          flexDir="column"
          onCheckboxChange={mockOnCheckboxChange}
        />,
      )

      // 세 번째 체크박스 선택 (이미 두 개가 선택된 상태)
      const checkbox3 = screen.getByLabelText('Option 3')

      // disabled 상태이므로 클릭해도 변화가 없어야 함
      fireEvent.click(checkbox3)

      // disabled 체크박스는 클릭되지 않으므로 호출되지 않아야 함
      expect(mockOnCheckboxChange).not.toHaveBeenCalled()
    })

    it('should not call onCheckboxChange when callback is not provided', () => {
      render(<CheckboxLayer checkboxes={mockCheckboxes} flexDir="column" />)

      const checkbox1 = screen.getByLabelText('Option 1')

      // 콜백이 없어도 에러가 발생하지 않아야 함
      expect(() => fireEvent.click(checkbox1)).not.toThrow()
    })
  })
})
