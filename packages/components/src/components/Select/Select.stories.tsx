import { Meta, StoryObj } from '@storybook/react-vite'

import { Select } from '.'

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
  args: {
    placeholder: 'Input text',
  },
}

export default meta
