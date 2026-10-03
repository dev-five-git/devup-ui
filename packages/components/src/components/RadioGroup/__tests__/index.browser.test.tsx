import { describe, expect, it, mock } from 'bun:test'
import { act, render, userEvent } from 'bun-test-env-dom'

import { RadioGroup } from '../index'

describe('RadioGroup', () => {
  const options = [
    { value: '1', label: '옵션 1' },
    { value: '2', label: '옵션 2' },
  ]

  it('should RadioGroup snapshot', () => {
    expect(
      render(<RadioGroup name="group" options={options} />).container,
    ).toMatchSnapshot()
    expect(
      render(<RadioGroup direction="column" name="group" options={options} />)
        .container,
    ).toMatchSnapshot()
    expect(
      render(<RadioGroup disabled name="group" options={options} />).container,
    ).toMatchSnapshot()
    expect(
      render(
        <RadioGroup
          direction="column"
          disabled
          name="group"
          options={options}
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(<RadioGroup name="group" options={options} variant="button" />)
        .container,
    ).toMatchSnapshot()
    expect(
      render(
        <RadioGroup disabled name="group" options={options} variant="button" />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <RadioGroup
          classNames={{ container: 'className', label: 'classNameLabel' }}
          name="group"
          options={options}
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <RadioGroup
          name="group"
          options={options}
          styles={{ container: { width: '500px' }, label: { width: '500px' } }}
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <RadioGroup
          colors={{
            primary: 'red',
            border: 'red',
            text: 'red',
            bg: 'red',
            hoverBg: 'red',
            hoverBorder: 'red',
            hoverColor: 'red',
            checkedBg: 'red',
            checkedBorder: 'red',
            checkedColor: 'red',
            disabledBg: 'red',
            disabledColor: 'red',
          }}
          name="group"
          options={options}
        />,
      ).container,
    ).toMatchSnapshot()
    expect(
      render(
        <RadioGroup
          colors={{
            primary: 'red',
            border: 'red',
            text: 'red',
            bg: 'red',
            hoverBg: 'red',
            hoverBorder: 'red',
            hoverColor: 'red',
            checkedBg: 'red',
            checkedBorder: 'red',
            checkedColor: 'red',
            disabledBg: 'red',
            disabledColor: 'red',
          }}
          name="group"
          options={options}
          variant="button"
        />,
      ).container,
    ).toMatchSnapshot()
  })
  it('should change value when click', async () => {
    const onChange = mock()
    const { getByText } = render(
      <RadioGroup defaultValue="1" onChange={onChange} options={options} />,
    )
    await act(async () => {
      await userEvent.click(getByText('옵션 2'))
      await userEvent.click(getByText('옵션 1'))
    })
    expect(onChange).toHaveBeenCalledTimes(2)
    expect(onChange).toHaveBeenNthCalledWith(1, '2')
    expect(onChange).toHaveBeenNthCalledWith(2, '1')
  })
  it('should have correct value with number values', async () => {
    const numberOptions = [
      { value: 1, label: '옵션 1' },
      { value: 2, label: '옵션 2' },
    ]
    const onChange = mock()
    const { getByText } = render(
      <RadioGroup
        defaultValue={1}
        onChange={onChange}
        options={numberOptions}
      />,
    )
    await act(async () => {
      await userEvent.click(getByText('옵션 2'))
      await userEvent.click(getByText('옵션 1'))
    })
    expect(onChange).toHaveBeenCalledTimes(2)
    expect(onChange).toHaveBeenNthCalledWith(1, 2)
    expect(onChange).toHaveBeenNthCalledWith(2, 1)
  })
  it('should have correct value with boolean values', async () => {
    const booleanOptions = [
      { value: true, label: '옵션 1' },
      { value: false, label: '옵션 2' },
    ]
    const onChange = mock()
    const { getByText } = render(
      <RadioGroup
        defaultValue={true}
        onChange={onChange}
        options={booleanOptions}
      />,
    )
    await act(async () => {
      await userEvent.click(getByText('옵션 2'))
      await userEvent.click(getByText('옵션 1'))
    })
    expect(onChange).toHaveBeenCalledTimes(2)
    expect(onChange).toHaveBeenNthCalledWith(1, false)
    expect(onChange).toHaveBeenNthCalledWith(2, true)
  })
  it('should have correct value with value prop', async () => {
    const onChange = mock()
    const { getByText } = render(
      <RadioGroup onChange={onChange} options={options} value="1" />,
    )
    await act(async () => {
      await userEvent.click(getByText('옵션 2'))
    })
    expect(onChange).toHaveBeenNthCalledWith(1, '2')
  })
  it('keeps a falsy value selected and groups its radios', () => {
    const { container, getByRole } = render(
      <RadioGroup
        defaultValue={0}
        label="Count"
        options={[
          { value: 0, label: 'zero' },
          { value: 1, label: 'one' },
        ]}
      />,
    )
    const radios =
      container.querySelectorAll<HTMLInputElement>('input[type=radio]')
    expect(radios[0].checked).toBe(true)
    expect(radios[0].name).toBe(radios[1].name)
    expect(getByRole('radiogroup')).toHaveAttribute('aria-label', 'Count')
  })

  it('uses the name it is given', () => {
    const { container } = render(
      <RadioGroup
        name="pick"
        options={[{ value: false, label: 'no' }]}
        value={false}
      />,
    )
    const radio =
      container.querySelector<HTMLInputElement>('input[type=radio]')!
    expect(radio.name).toBe('pick')
    expect(radio.checked).toBe(true)
  })
})
