import { Meta, StoryObj } from '@storybook/react-vite'

import { Checkbox } from '.'

type Story = StoryObj<typeof meta>

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Checkbox> = {
  title: 'Devfive/Checkbox',
  component: Checkbox,
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
    children: 'Checkbox',
    disabled: false,
  },
}

export default meta
