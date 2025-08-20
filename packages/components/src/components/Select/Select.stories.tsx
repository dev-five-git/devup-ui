import { css, Flex } from '@devup-ui/react'
import { Meta, StoryObj } from '@storybook/react-vite'
import { ComponentProps, useState } from 'react'

import {
  Select,
  SelectContainer,
  SelectDivider,
  SelectOption,
  SelectTrigger,
} from '.'
import { IconArrow } from './IconArrow'

export type Story = StoryObj<typeof meta>

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Select> = {
  title: 'Devfive/Select',
  component: Select,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px' }}>
        <Story />
      </div>
    ),
  ],
}

export const DefaultStory: Story = {
  args: {
    colors: {},
  },
  render: (args) => <DefaultComponent {...args} />,
}

export const ControlledRadioStory: Story = {
  args: {},
  render: () => <ControlledRadio />,
}

export const ControlledCheckboxStory: Story = {
  args: {},
  render: () => <ControlledCheckbox />,
}

export default meta

function DefaultComponent(
  props: Omit<ComponentProps<typeof Select>, 'children'>,
) {
  return (
    <Select {...props} defaultValue={['Option 1']} onValueChange={() => {}}>
      <SelectTrigger>Select</SelectTrigger>
      <SelectContainer>
        <SelectOption disabled value="Option 1">
          Option 1
        </SelectOption>
        <SelectOption value="Option 2">Option 2</SelectOption>
        <SelectDivider />
        <SelectOption value="Option 3">Option 3</SelectOption>
        <SelectOption disabled value="Option 4">
          Option 4
        </SelectOption>
        <Select type="radio">
          <SelectTrigger asChild>
            <SelectOption>
              <Flex alignItems="center" justifyContent="space-between" w="100%">
                Option 5<IconArrow />
              </Flex>
            </SelectOption>
          </SelectTrigger>
          <SelectContainer
            className={css({
              right: '0',
              top: '0',
              transform: 'translateX(100%)',
            })}
          >
            <SelectOption value="Option 6">Option 6</SelectOption>
            <SelectOption value="Option 7">Option 7</SelectOption>
          </SelectContainer>
        </Select>
      </SelectContainer>
    </Select>
  )
}

function ControlledCheckbox() {
  const [value, setValue] = useState<string[]>([])
  const handleChange = (nextValue: string) => {
    if (value.includes(nextValue)) {
      setValue(value.filter((v) => v !== nextValue))
    } else {
      setValue([...value, nextValue])
    }
  }

  const [subValue, setSubValue] = useState<string[]>([])
  const handleSubChange = (nextValue: string) => {
    if (subValue.includes(nextValue)) {
      setSubValue(subValue.filter((v) => v !== nextValue))
    } else {
      setSubValue([...subValue, nextValue])
    }
  }

  return (
    <Select onValueChange={handleChange} type="checkbox" value={value}>
      <SelectTrigger>Select {value}</SelectTrigger>
      <SelectContainer showConfirmButton>
        <SelectOption value="Option 1">Option 1</SelectOption>
        <SelectOption value="Option 2">Option 2</SelectOption>
        <SelectDivider />
        <SelectOption value="Option 3">Option 3</SelectOption>
        <SelectOption value="Option 4">Option 4</SelectOption>
        <Select
          onValueChange={handleSubChange}
          type="checkbox"
          value={subValue}
        >
          <SelectTrigger asChild>
            <SelectOption showCheck={false}>
              <Flex alignItems="center" justifyContent="space-between" w="100%">
                Option 5<IconArrow />
              </Flex>
            </SelectOption>
          </SelectTrigger>
          <SelectContainer
            className={css({
              right: '0',
              top: '0',
              transform: 'translateX(100%)',
            })}
          >
            <SelectOption value="Option 6">Option 6</SelectOption>
            <SelectOption value="Option 7">Option 7</SelectOption>
          </SelectContainer>
        </Select>
      </SelectContainer>
    </Select>
  )
}

function ControlledRadio() {
  const [value, setValue] = useState('')
  const handleChange = (value: string) => {
    setValue(value)
  }
  const [subValue, setSubValue] = useState('')
  const handleSubChange = (value: string) => {
    setSubValue(value)
  }
  return (
    <Select onValueChange={handleChange} type="radio" value={value}>
      <SelectTrigger>Select {value}</SelectTrigger>
      <SelectContainer>
        <SelectOption value="Option 1">Option 1</SelectOption>
        <SelectOption value="Option 2">Option 2</SelectOption>
        <SelectDivider />
        <SelectOption value="Option 3">Option 3</SelectOption>
        <SelectOption value="Option 4">Option 4</SelectOption>
        <Select onValueChange={handleSubChange} type="radio" value={subValue}>
          <SelectTrigger asChild>
            <SelectOption showCheck={false}>
              <Flex alignItems="center" justifyContent="space-between" w="100%">
                Option 5<IconArrow />
              </Flex>
            </SelectOption>
          </SelectTrigger>
          <SelectContainer
            className={css({
              right: '0',
              top: '0',
              transform: 'translateX(100%)',
            })}
          >
            <SelectOption
              onClick={(value) => {
                if (value) {
                  setSubValue(value)
                }
              }}
              value="Option 6"
            >
              Option 6
            </SelectOption>
            <SelectOption
              onClick={(value) => {
                if (value) {
                  setSubValue(value)
                }
              }}
              value="Option 7"
            >
              Option 7
            </SelectOption>
          </SelectContainer>
        </Select>
      </SelectContainer>
    </Select>
  )
}
