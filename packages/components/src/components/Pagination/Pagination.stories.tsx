import { Meta, StoryObj } from '@storybook/react-vite'
import { useState } from 'react'

import {
  Pagination,
  PaginationContainer,
  PaginationNextButton,
  PaginationPages,
  PaginationPrevButton,
} from './index'

type Story = StoryObj<typeof meta>

const meta: Meta<typeof Pagination> = {
  title: 'Devfive/Pagination',
  component: Pagination,
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
    totalPages: 10,
    defaultPage: 1,
  },
}

export const PropsBasedSimple: Story = {
  args: {
    totalPages: 10,
    defaultPage: 1,
  },
  render: (args) => <Pagination {...args} />,
}

export const WithoutPrevNext: Story = {
  args: {
    totalPages: 10,
    defaultPage: 1,
    showPrevNext: false,
  },
  render: (args) => <Pagination {...args} />,
}

export const CompositionBased: Story = {
  args: {
    totalPages: 10,
    defaultPage: 1,
  },
  render: (args) => (
    <Pagination {...args}>
      <PaginationContainer>
        <PaginationPrevButton />
        <PaginationPages />
        <PaginationNextButton />
      </PaginationContainer>
    </Pagination>
  ),
}

export const Controlled: Story = {
  render: () => {
    const [currentPage, setCurrentPage] = useState(1)

    return (
      <div>
        <div style={{ marginBottom: '10px' }}>Current Page: {currentPage}</div>
        <Pagination
          currentPage={currentPage}
          onPageChange={setCurrentPage}
          totalPages={15}
        />
      </div>
    )
  },
}

export const ManyPages: Story = {
  args: {
    totalPages: 100,
    defaultPage: 50,
  },
}

export const FewPages: Story = {
  args: {
    totalPages: 5,
    defaultPage: 1,
  },
}

export const WithoutFirstLast: Story = {
  args: {
    totalPages: 20,
    defaultPage: 10,
    showFirstLast: false,
  },
}

export const CustomClassName: Story = {
  args: {
    totalPages: 10,
    defaultPage: 1,
    className: 'custom-pagination',
  },
  render: (args) => (
    <div>
      <style>{`.custom-pagination { padding: 10px; background: rgba(129, 99, 225, 0.1); border-radius: 8px; }`}</style>
      <Pagination {...args} />
    </div>
  ),
}

export default meta
