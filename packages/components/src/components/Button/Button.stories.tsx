import { Meta } from '@storybook/react-vite'

import { Button } from './index'

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Button> = {
  title: 'Devfive/Button',
  component: Button,
  decorators: [
    (Story) => (
      <div style={{ padding: '10px' }}>
        <Story />
      </div>
    ),
  ],
}

export const Default = {
  args: {
    children: 'Button Text',
    variant: 'default',
    disabled: false,
    colors: {
      primary: 'var(--primary)',
      error: 'var(--error)',
    },
    isError: false,
    size: 'm',
  },
}

export default meta
