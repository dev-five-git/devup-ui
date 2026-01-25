import { Meta, StoryObj } from '@storybook/react-vite'

import { Textarea } from './index'

type Story = StoryObj<typeof meta>

const meta: Meta<typeof Textarea> = {
  title: 'Devfive/Textarea',
  component: Textarea,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px', maxWidth: '400px' }}>
        <Story />
      </div>
    ),
  ],
}

export const Default: Story = {
  args: {
    placeholder: 'Enter your message...',
  },
}

export const WithDefaultValue: Story = {
  args: {
    defaultValue: 'This is some default text in the textarea.',
    placeholder: 'Enter your message...',
  },
}

export const Disabled: Story = {
  args: {
    placeholder: 'Disabled textarea',
    disabled: true,
  },
}

export const DisabledWithValue: Story = {
  args: {
    defaultValue: 'This textarea is disabled',
    disabled: true,
  },
}

export const Error: Story = {
  args: {
    placeholder: 'Enter your message...',
    error: true,
  },
}

export const ErrorWithMessage: Story = {
  args: {
    placeholder: 'Enter your message...',
    error: true,
    errorMessage: 'Please enter a valid message.',
  },
}

export const CustomRows: Story = {
  args: {
    placeholder: 'This textarea has 6 rows',
    rows: 6,
  },
}

export const CustomColors: Story = {
  args: {
    placeholder: 'Custom themed textarea',
    colors: {
      primary: '#8B5CF6',
      border: '#E5E7EB',
      background: '#F9FAFB',
    },
  },
}

export default meta
