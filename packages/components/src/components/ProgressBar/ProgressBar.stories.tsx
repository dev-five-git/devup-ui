import { Meta, StoryObj } from '@storybook/react-vite'

import { ProgressBar } from './index'

type Story = StoryObj<typeof meta>

const meta: Meta<typeof ProgressBar> = {
  title: 'Devfive/ProgressBar',
  component: ProgressBar,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px', width: '400px' }}>
        <Story />
      </div>
    ),
  ],
}

export const Default: Story = {
  args: {
    'aria-label': 'Progress',
    showValue: true,
    value: 40,
  },
}

export const Step: Story = {
  args: {
    'aria-label': 'Steps',
    max: 6,
    showValue: true,
    value: 3,
    variant: 'step',
  },
}

export default meta
