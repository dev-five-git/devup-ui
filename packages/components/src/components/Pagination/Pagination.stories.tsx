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
        >
          <PaginationContainer>
            <PaginationPrevButton />
            <PaginationPages />
            <PaginationNextButton />
          </PaginationContainer>
        </Pagination>
      </div>
    )
  },
}

export const ManyPages: Story = {
  args: {
    totalPages: 100,
    defaultPage: 50,
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

export const FewPages: Story = {
  args: {
    totalPages: 5,
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

export const WithoutFirstLast: Story = {
  args: {
    totalPages: 20,
    defaultPage: 10,
    showFirstLast: false,
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

export default meta
