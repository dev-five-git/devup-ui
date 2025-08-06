import { Meta, StoryObj } from '@storybook/react-vite'

import { GlassIcon } from './GlassIcon'
import { Input } from './index'

type Story = StoryObj<typeof meta>

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Input> = {
  title: 'Devfive/Input',
  component: Input,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px' }}>
        <Story />
      </div>
    ),
  ],
}

export const Default: Story = {
  args: {
    colors: {
      primary: 'red',
      error: 'blue',
      text: 'green',
    },
    placeholder: 'Input text',
  },
}

export const Error: Story = {
  args: {
    placeholder: 'Input text',
    error: true,
    errorMessage: 'Error message',
  },
}

export const Disabled: Story = {
  args: {
    placeholder: 'Input text',
    disabled: true,
  },
}

export const WithIcon: Story = {
  args: {
    placeholder: 'Input text',
    allowClear: true,
    icon: <GlassIcon />,
  },
}

export default meta
