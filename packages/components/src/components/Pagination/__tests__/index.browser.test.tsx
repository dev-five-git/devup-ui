import { fireEvent, render } from '@testing-library/react'

import {
  Pagination,
  PaginationContainer,
  PaginationNextButton,
  PaginationPageButton,
  PaginationPages,
  PaginationPrevButton,
} from '..'

describe('Pagination', () => {
  it('should render', () => {
    const { container } = render(
      <Pagination totalPages={10}>
        <PaginationContainer>
          <PaginationPrevButton />
          <PaginationPages />
          <PaginationNextButton />
        </PaginationContainer>
      </Pagination>,
    )
    expect(container).toMatchSnapshot()
  })

  it('should throw error if children are used outside of Pagination', () => {
    expect(() => {
      render(<PaginationPrevButton />)
    }).toThrow('usePagination must be used within a Pagination component')
  })

  it('should call onPageChange when page is changed', () => {
    const onPageChange = vi.fn()
    const { container } = render(
      <Pagination onPageChange={onPageChange} totalPages={10}>
        <PaginationContainer>
          <PaginationPageButton page={2} />
        </PaginationContainer>
      </Pagination>,
    )
    const button = container.querySelector('[aria-label="Page 2"]')
    fireEvent.click(button!)
    expect(onPageChange).toHaveBeenCalledWith(2)
  })

  it('should change internal page when onPageChange is not provided', () => {
    const { container } = render(
      <Pagination totalPages={10}>
        <PaginationContainer>
          <PaginationPageButton page={2} />
        </PaginationContainer>
      </Pagination>,
    )
    const button = container.querySelector('[aria-label="Page 2"]')
    fireEvent.click(button!)
    expect(button).toHaveAttribute('aria-current', 'page')
  })

  it('should disable prev button when on first page', () => {
    const { container } = render(
      <Pagination defaultPage={1} totalPages={10}>
        <PaginationContainer>
          <PaginationPrevButton />
        </PaginationContainer>
      </Pagination>,
    )
    const prevButton = container.querySelector('[aria-label="Previous page"]')
    expect(prevButton).toHaveAttribute('disabled')
  })

  it('should disable next button when on last page', () => {
    const { container } = render(
      <Pagination defaultPage={10} totalPages={10}>
        <PaginationContainer>
          <PaginationNextButton />
        </PaginationContainer>
      </Pagination>,
    )
    const nextButton = container.querySelector('[aria-label="Next page"]')
    expect(nextButton).toHaveAttribute('disabled')
  })

  it('should navigate to next page when next button is clicked', () => {
    const onPageChange = vi.fn()
    const { container } = render(
      <Pagination
        currentPage={1}
        onPageChange={onPageChange}
        totalPages={10}
      >
        <PaginationContainer>
          <PaginationNextButton />
        </PaginationContainer>
      </Pagination>,
    )
    const nextButton = container.querySelector('[aria-label="Next page"]')
    fireEvent.click(nextButton!)
    expect(onPageChange).toHaveBeenCalledWith(2)
  })

  it('should navigate to previous page when prev button is clicked', () => {
    const onPageChange = vi.fn()
    const { container } = render(
      <Pagination
        currentPage={5}
        onPageChange={onPageChange}
        totalPages={10}
      >
        <PaginationContainer>
          <PaginationPrevButton />
        </PaginationContainer>
      </Pagination>,
    )
    const prevButton = container.querySelector('[aria-label="Previous page"]')
    fireEvent.click(prevButton!)
    expect(onPageChange).toHaveBeenCalledWith(4)
  })

  it('should render all pages when total is 7 or less', () => {
    const { container } = render(
      <Pagination totalPages={5}>
        <PaginationPages />
      </Pagination>,
    )
    const buttons = container.querySelectorAll('[aria-label^="Page"]')
    expect(buttons.length).toBe(5)
  })

  it('should render ellipsis for many pages', () => {
    const { container } = render(
      <Pagination defaultPage={1} totalPages={20}>
        <PaginationPages />
      </Pagination>,
    )
    const ellipsis = container.querySelector('[aria-hidden="true"]')
    expect(ellipsis).toBeInTheDocument()
    expect(ellipsis?.textContent).toBe('...')
  })

  it('should mark active page button', () => {
    const { container } = render(
      <Pagination currentPage={3} totalPages={10}>
        <PaginationPageButton page={3} />
      </Pagination>,
    )
    const button = container.querySelector('[aria-label="Page 3"]')
    expect(button).toHaveAttribute('aria-current', 'page')
  })

  it('should export components', async () => {
    const index = await import('../index')
    expect({ ...index }).toEqual({
      Pagination: expect.any(Function),
      PaginationContainer: expect.any(Function),
      PaginationEllipsis: expect.any(Function),
      PaginationNextButton: expect.any(Function),
      PaginationPageButton: expect.any(Function),
      PaginationPages: expect.any(Function),
      PaginationPrevButton: expect.any(Function),
      usePagination: expect.any(Function),
    })
  })

  it('should not exceed totalPages when navigating', () => {
    const onPageChange = vi.fn()
    const { container } = render(
      <Pagination
        currentPage={10}
        onPageChange={onPageChange}
        totalPages={10}
      >
        <PaginationContainer>
          <PaginationNextButton />
        </PaginationContainer>
      </Pagination>,
    )
    const nextButton = container.querySelector('[aria-label="Next page"]')
    expect(nextButton).toHaveAttribute('disabled')
  })

  it('should not go below 1 when navigating', () => {
    const onPageChange = vi.fn()
    const { container } = render(
      <Pagination
        currentPage={1}
        onPageChange={onPageChange}
        totalPages={10}
      >
        <PaginationContainer>
          <PaginationPrevButton />
        </PaginationContainer>
      </Pagination>,
    )
    const prevButton = container.querySelector('[aria-label="Previous page"]')
    expect(prevButton).toHaveAttribute('disabled')
  })
})
