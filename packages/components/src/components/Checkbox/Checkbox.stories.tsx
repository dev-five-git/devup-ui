import { Meta, StoryObj } from '@storybook/react-vite'

import { Checkbox } from './index'

type Story = StoryObj<typeof meta>

// More on how to set up stories at: https://storybook.js.org/docs/writing-stories#default-export
const meta: Meta<typeof Checkbox> = {
  title: 'Devfive/Checkbox',
  component: Checkbox,
  decorators: [
    (Story, context) => {
      const theme =
        context.parameters.theme || context.globals.theme || 'default'
      const isDark = theme === 'dark'

      return (
        <div
          data-theme={theme}
          style={{
            padding: '20px',
            backgroundColor: isDark ? '#1a1a1a' : '#ffffff',
            color: isDark ? '#ffffff' : '#000000',
            minHeight: '200px',
          }}
        >
          <Story />
        </div>
      )
    },
  ],
}

export const Default: Story = {
  args: {
    children: 'Checkbox',
    disabled: false,
    checked: true,
  },
}

export default meta
