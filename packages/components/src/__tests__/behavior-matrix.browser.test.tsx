import { describe, expect, it, mock } from 'bun:test'
import { act, render, userEvent } from 'bun-test-env-dom'
import type { ReactElement } from 'react'

import {
  Button,
  Checkbox,
  Input,
  RadioGroup,
  Select,
  SelectContainer,
  SelectOption,
  SelectTrigger,
  Stepper,
  StepperContainer,
  StepperDecreaseButton,
  StepperIncreaseButton,
  StepperInput,
  Textarea,
  Toggle,
} from '../index'

// The user-facing contract of every interactive component, as a matrix: each
// concern below runs for each component row. A concern covers what assistive
// technology announces (role, name, description), how the keyboard reaches
// and operates the control, and what disabling takes away. Every interaction
// goes through user-event, so focus moves the way a user moves it.

interface Row {
  component: string
  role: string
  name: string
  render: (disabled: boolean, onChange: () => void) => ReactElement
  /** Keys that operate the focused control, and the state they must leave */
  keys: { press: string; expect: (control: HTMLElement) => void }
}

const rows: Row[] = [
  {
    component: 'Button',
    role: 'button',
    name: 'Save',
    render: (disabled, onChange) => (
      <Button disabled={disabled} onClick={onChange}>
        Save
      </Button>
    ),
    keys: { press: '{Enter}', expect: () => {} },
  },
  {
    component: 'Checkbox',
    role: 'checkbox',
    name: 'Agree',
    render: (disabled, onChange) => (
      <Checkbox disabled={disabled} onChange={onChange}>
        Agree
      </Checkbox>
    ),
    keys: { press: ' ', expect: (control) => expect(control).toBeChecked() },
  },
  {
    component: 'Toggle',
    role: 'switch',
    name: 'Dark mode',
    render: (disabled, onChange) => (
      <Toggle aria-label="Dark mode" disabled={disabled} onChange={onChange} />
    ),
    keys: {
      press: ' ',
      expect: (control) =>
        expect(control).toHaveAttribute('aria-checked', 'true'),
    },
  },
  {
    component: 'RadioGroup',
    role: 'radio',
    name: 'Small',
    render: (disabled, onChange) => (
      <RadioGroup
        disabled={disabled}
        label="Size"
        onChange={onChange}
        options={[
          { value: 's', label: 'Small' },
          { value: 'l', label: 'Large' },
        ]}
      />
    ),
    keys: { press: ' ', expect: (control) => expect(control).toBeChecked() },
  },
  {
    component: 'Input',
    role: 'textbox',
    name: 'Email',
    render: (disabled, onChange) => (
      <Input aria-label="Email" disabled={disabled} onChange={onChange} />
    ),
    keys: {
      press: 'a',
      expect: (control) => expect(control).toHaveValue('a'),
    },
  },
  {
    component: 'Textarea',
    role: 'textbox',
    name: 'Message',
    render: (disabled, onChange) => (
      <Textarea aria-label="Message" disabled={disabled} onChange={onChange} />
    ),
    keys: {
      press: 'a',
      expect: (control) => expect(control).toHaveValue('a'),
    },
  },
  {
    component: 'Select',
    role: 'button',
    name: 'Fruit',
    render: (disabled, onChange) => (
      <Select onOpenChange={onChange} type="radio">
        <SelectTrigger disabled={disabled}>Fruit</SelectTrigger>
        <SelectContainer>
          <SelectOption value="apple">Apple</SelectOption>
        </SelectContainer>
      </Select>
    ),
    keys: {
      press: '{Enter}',
      expect: (control) =>
        expect(control).toHaveAttribute('aria-expanded', 'true'),
    },
  },
  {
    component: 'StepperIncreaseButton',
    role: 'button',
    name: 'Increase button',
    render: (disabled, onChange) => (
      <Stepper onValueChange={onChange}>
        <StepperContainer>
          <StepperIncreaseButton disabled={disabled} />
        </StepperContainer>
      </Stepper>
    ),
    keys: { press: '{Enter}', expect: () => {} },
  },
]

/** Render a row between two native buttons, so Tab has a start and an end */
function between(row: Row, disabled = false) {
  const onChange = mock()
  const view = render(
    <>
      <button type="button">before</button>
      {row.render(disabled, onChange)}
      <button type="button">after</button>
    </>,
  )
  const control = view.getByRole(row.role, { name: row.name })
  return { ...view, control, onChange }
}

async function press(keys: string) {
  await act(async () => {
    await userEvent.keyboard(keys)
  })
}

async function tab(shift = false) {
  await act(async () => {
    await userEvent.tab({ shift })
  })
}

describe.each(rows)('$component', (row) => {
  it('exposes its role and accessible name', () => {
    const { control } = between(row)
    expect(control).toHaveAccessibleName(row.name)
  })

  it('is reached by Tab and left by Shift+Tab', async () => {
    const { control, getByRole } = between(row)
    getByRole('button', { name: 'before' }).focus()
    await tab()
    expect(control).toHaveFocus()
    await tab(true)
    expect(getByRole('button', { name: 'before' })).toHaveFocus()
  })

  it('is operated by the keyboard once focused', async () => {
    const { control, onChange } = between(row)
    control.focus()
    await press(row.keys.press)
    row.keys.expect(control)
    expect(onChange).toHaveBeenCalled()
  })

  it('leaves the tab order and ignores the keyboard when disabled', async () => {
    const { control, getByRole, onChange } = between(row, true)
    getByRole('button', { name: 'before' }).focus()
    await tab()
    expect(control).not.toHaveFocus()
    expect(getByRole('button', { name: 'after' })).toHaveFocus()
    control.focus()
    await press(row.keys.press)
    expect(onChange).not.toHaveBeenCalled()
    expect(control).not.toHaveAttribute('aria-expanded', 'true')
  })
})

describe.each([
  ['Input', Input],
  ['Textarea', Textarea],
] as const)('%s description', (_, Field) => {
  it('announces the error message with its own description', () => {
    const { getByRole } = render(
      <>
        <p id="hint">We never share it</p>
        <Field
          aria-describedby="hint"
          aria-label="Email"
          error
          errorMessage="Enter an email"
        />
      </>,
    )
    const field = getByRole('textbox', { name: 'Email' })
    expect(field).toHaveAccessibleDescription(
      'We never share it Enter an email',
    )
    expect(field).toBeInvalid()
    expect(getByRole('alert')).toHaveTextContent('Enter an email')
  })

  it('announces no error before there is one', () => {
    const { getByRole, queryByRole } = render(
      <Field aria-label="Email" errorMessage="Enter an email" />,
    )
    expect(
      getByRole('textbox', { name: 'Email' }),
    ).not.toHaveAccessibleDescription()
    expect(queryByRole('alert')).toBeNull()
  })
})

describe('RadioGroup group', () => {
  it('names the group and moves the selection with the arrow keys', async () => {
    const onChange = mock()
    const { getByRole } = render(
      <RadioGroup
        label="Size"
        onChange={onChange}
        options={[
          { value: 's', label: 'Small' },
          { value: 'm', label: 'Medium' },
          { value: 'l', label: 'Large' },
        ]}
      />,
    )
    expect(getByRole('radiogroup', { name: 'Size' })).toBeInTheDocument()
    const small = getByRole('radio', { name: 'Small' })
    small.focus()
    await press(' ')
    expect(small).toBeChecked()
    await press('{ArrowRight}')
    expect(getByRole('radio', { name: 'Medium' })).toBeChecked()
    expect(getByRole('radio', { name: 'Medium' })).toHaveFocus()
    expect(onChange).toHaveBeenLastCalledWith('m')
  })
})

describe('Select keyboard path', () => {
  function renderSelect(onChange = mock()) {
    const view = render(
      <>
        <button type="button">before</button>
        <Select onChange={onChange} type="radio">
          <SelectTrigger>Fruit</SelectTrigger>
          <SelectContainer>
            <SelectOption value="apple">Apple</SelectOption>
            <SelectOption disabled value="banana">
              Banana
            </SelectOption>
            <SelectOption value="cherry">Cherry</SelectOption>
          </SelectContainer>
        </Select>
      </>,
    )
    return {
      ...view,
      onChange,
      trigger: view.getByRole('button', { name: 'Fruit' }),
    }
  }

  it('opens from Tab and Enter, names its listbox and focuses the first option', async () => {
    const { getByRole, trigger } = renderSelect()
    getByRole('button', { name: 'before' }).focus()
    await tab()
    expect(trigger).toHaveFocus()
    expect(trigger).toHaveAttribute('aria-haspopup', 'listbox')
    await press('{Enter}')
    expect(trigger).toHaveAttribute('aria-expanded', 'true')
    expect(getByRole('listbox', { name: 'Fruit' })).toBeInTheDocument()
    expect(getByRole('option', { name: 'Apple' })).toHaveFocus()
    expect(getByRole('option', { name: 'Banana' })).toHaveAttribute(
      'aria-disabled',
      'true',
    )
  })

  it('skips a disabled option, selects with Enter and gives focus back', async () => {
    const { getByRole, onChange, queryByRole, trigger } = renderSelect()
    trigger.focus()
    await press('{Enter}')
    await press('{ArrowDown}')
    expect(getByRole('option', { name: 'Cherry' })).toHaveFocus()
    await press('{Enter}')
    expect(onChange).toHaveBeenCalledWith('cherry')
    expect(queryByRole('listbox')).toBeNull()
    expect(trigger).toHaveFocus()
    expect(trigger).toHaveAttribute('aria-expanded', 'false')
    await press('{Enter}')
    expect(getByRole('option', { name: 'Cherry' })).toHaveFocus()
    expect(getByRole('option', { name: 'Cherry' })).toHaveAttribute(
      'aria-selected',
      'true',
    )
  })

  it('closes on Escape without choosing and gives focus back', async () => {
    const { onChange, queryByRole, trigger } = renderSelect()
    trigger.focus()
    await press('{Enter}')
    await press('{Escape}')
    expect(queryByRole('listbox')).toBeNull()
    expect(trigger).toHaveFocus()
    expect(onChange).not.toHaveBeenCalled()
  })
})

describe('Stepper', () => {
  it('names its buttons and value, and stops at its bounds from the keyboard', async () => {
    const onValueChange = mock()
    const { getByRole } = render(
      <Stepper max={1} onValueChange={onValueChange}>
        <StepperContainer>
          <StepperDecreaseButton />
          <StepperInput />
          <StepperIncreaseButton />
        </StepperContainer>
      </Stepper>,
    )
    const decrease = getByRole('button', { name: 'Decrease button' })
    const increase = getByRole('button', { name: 'Increase button' })
    const value = getByRole('spinbutton', { name: 'Stepper value' })
    expect(decrease).toBeDisabled()
    increase.focus()
    await press('{Enter}')
    expect(onValueChange).toHaveBeenLastCalledWith(1)
    expect(value).toHaveValue(1)
    expect(increase).toBeDisabled()
    expect(decrease).toBeEnabled()
  })
})
