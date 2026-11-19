import { Meta, StoryObj } from '@storybook/react-vite'

import { Search } from './index'

type Story = StoryObj<typeof meta>

const meta: Meta<typeof Search> = {
  title: 'Devfive/Search',
  component: Search,
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
    'aria-label': 'Search',
    placeholder: 'Search',
  },
}

export const WithoutIcon: Story = {
  args: {
    'aria-label': 'Search',
    icon: null,
    placeholder: 'Search',
  },
}

export const Disabled: Story = {
  args: {
    'aria-label': 'Search',
    disabled: true,
    placeholder: 'Search',
  },
}

export default meta
