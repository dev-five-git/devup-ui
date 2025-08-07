import { css, Flex } from '@devup-ui/react'
import { Meta, StoryObj } from '@storybook/react-vite'

import {
  Select,
  SelectContainer,
  SelectDivider,
  SelectOption,
  SelectTrigger,
} from '.'
import { ControlledCheckbox } from './ControlledCheckbox'
import { ControlledRadio } from './ControlledRadio'
import { IconArrow } from './IconArrow'

type Story = StoryObj<typeof meta>

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

export const Default: Story = {
  args: { type: 'radio' },
  render: (args) => (
    <Select {...args} defaultValue="Option 1">
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
  ),
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
