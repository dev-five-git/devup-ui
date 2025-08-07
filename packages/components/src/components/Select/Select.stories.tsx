import { Meta, StoryObj } from '@storybook/react-vite'

import {
  Select,
  SelectContainer,
  SelectDivider,
  SelectOption,
  SelectTrigger,
} from '.'

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
  args: {},
  render: (args) => (
    <Select {...args}>
      <SelectTrigger>Select</SelectTrigger>
      <SelectContainer>
        <SelectOption>Option 1</SelectOption>
        <SelectOption>Option 2</SelectOption>
        <SelectDivider />
        <SelectOption>Option 3</SelectOption>
        <SelectOption disabled>Option 4</SelectOption>
      </SelectContainer>
    </Select>
  ),
}

export default meta
