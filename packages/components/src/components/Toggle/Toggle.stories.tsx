import { Meta, StoryObj } from '@storybook/react-vite'

import { Toggle } from './index'

type Story = StoryObj<typeof meta>

const meta: Meta<typeof Toggle> = {
  title: 'Devfive/Toggle',
  component: Toggle,
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
    defaultValue: false,
    variant: 'default',
    disabled: false,
  },
}

export default meta
