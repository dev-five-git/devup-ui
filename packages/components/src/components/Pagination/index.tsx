'use client'

import { css, Flex } from '@devup-ui/react'
import clsx from 'clsx'
import { ComponentProps, createContext, useContext, useMemo, useState } from 'react'

import { Button } from '../Button'
import { IconChevronLeft } from './IconChevronLeft'
import { IconChevronRight } from './IconChevronRight'

type PaginationContextType = {
  currentPage: number
  totalPages: number
  setPage: (page: number) => void
  siblingCount: number
  showFirstLast: boolean
}

const PaginationContext = createContext<PaginationContextType | null>(null)

export const usePagination = () => {
  const context = useContext(PaginationContext)
  if (!context) {
    throw new Error('usePagination must be used within a Pagination component')
  }
  return context
}

type PaginationProps = {
  children?: React.ReactNode
  defaultPage?: number
  currentPage?: number
  totalPages: number
  onPageChange?: (page: number) => void
  siblingCount?: number
  showFirstLast?: boolean
}

function Pagination({
  children,
  defaultPage = 1,
  currentPage: currentPageProp,
  totalPages,
  onPageChange,
  siblingCount = 1,
  showFirstLast = true,
}: PaginationProps) {
  const [internalPage, setInternalPage] = useState(defaultPage)

  const currentPage = currentPageProp ?? internalPage

  const handlePageChange = (page: number) => {
    const sanitizedPage = Math.min(Math.max(page, 1), totalPages)
    if (onPageChange) {
      onPageChange(sanitizedPage)
    } else {
      setInternalPage(sanitizedPage)
    }
  }

  return (
    <PaginationContext.Provider
      value={{
        currentPage,
        totalPages,
        setPage: handlePageChange,
        siblingCount,
        showFirstLast,
      }}
    >
      {children}
    </PaginationContext.Provider>
  )
}

function PaginationContainer({ className, ...props }: ComponentProps<'div'>) {
  return (
    <Flex
      alignItems="center"
      className={clsx(
        css({
          gap: '4px',
          styleOrder: 1,
        }),
        className,
      )}
      {...props}
    />
  )
}

function PaginationPrevButton({
  className,
  ...props
}: ComponentProps<typeof Button>) {
  const { currentPage, setPage } = usePagination()
  const disabled = currentPage <= 1

  return (
    <Button
      aria-label="Previous page"
      className={clsx(
        css({
          p: '0',
          minW: '32px',
          h: '32px',
          borderRadius: '6px',
          styleOrder: 2,
        }),
        className,
      )}
      disabled={disabled}
      onClick={() => setPage(currentPage - 1)}
      {...props}
    >
      <IconChevronLeft
        className={css({
          color: disabled
            ? 'var(--base10, light-dark(#0000001A,#FFFFFF1A))'
            : 'var(--text, light-dark(#272727, #F6F6F6))',
        })}
      />
    </Button>
  )
}

function PaginationNextButton({
  className,
  ...props
}: ComponentProps<typeof Button>) {
  const { currentPage, totalPages, setPage } = usePagination()
  const disabled = currentPage >= totalPages

  return (
    <Button
      aria-label="Next page"
      className={clsx(
        css({
          p: '0',
          minW: '32px',
          h: '32px',
          borderRadius: '6px',
          styleOrder: 2,
        }),
        className,
      )}
      disabled={disabled}
      onClick={() => setPage(currentPage + 1)}
      {...props}
    >
      <IconChevronRight
        className={css({
          color: disabled
            ? 'var(--base10, light-dark(#0000001A,#FFFFFF1A))'
            : 'var(--text, light-dark(#272727, #F6F6F6))',
        })}
      />
    </Button>
  )
}

type PaginationPageButtonProps = ComponentProps<typeof Button> & {
  page: number
}

function PaginationPageButton({
  page,
  className,
  ...props
}: PaginationPageButtonProps) {
  const { currentPage, setPage } = usePagination()
  const isActive = currentPage === page

  return (
    <Button
      aria-current={isActive ? 'page' : undefined}
      aria-label={`Page ${page}`}
      className={clsx(
        css({
          p: '0',
          minW: '32px',
          h: '32px',
          borderRadius: '6px',
          fontSize: '14px',
          fontWeight: 600,
          styleOrder: 2,
        }),
        className,
      )}
      onClick={() => setPage(page)}
      variant={isActive ? 'primary' : 'default'}
      {...props}
    >
      {page}
    </Button>
  )
}

function PaginationEllipsis({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      aria-hidden="true"
      className={clsx(
        css({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minW: '32px',
          h: '32px',
          color: 'var(--text, light-dark(#272727, #F6F6F6))',
          fontSize: '14px',
        }),
        className,
      )}
      {...props}
    >
      ...
    </div>
  )
}

function PaginationPages({ className, ...props }: ComponentProps<'div'>) {
  const { currentPage, totalPages, siblingCount, showFirstLast } =
    usePagination()

  const pageNumbers = useMemo(() => {
    const pages: (number | 'ellipsis')[] = []

    if (totalPages <= 7) {
      // Show all pages if total is 7 or less
      for (let i = 1; i <= totalPages; i++) {
        pages.push(i)
      }
      return pages
    }

    // Always show first page
    if (showFirstLast) {
      pages.push(1)
    }

    const leftSiblingIndex = Math.max(currentPage - siblingCount, 1)
    const rightSiblingIndex = Math.min(currentPage + siblingCount, totalPages)

    const shouldShowLeftEllipsis = leftSiblingIndex > 2
    const shouldShowRightEllipsis = rightSiblingIndex < totalPages - 1

    if (!shouldShowLeftEllipsis && shouldShowRightEllipsis) {
      // No left ellipsis, show right ellipsis
      for (let i = 1; i <= Math.min(5, totalPages - 1); i++) {
        if (!showFirstLast || i > 1) {
          pages.push(i)
        }
      }
      pages.push('ellipsis')
    } else if (shouldShowLeftEllipsis && !shouldShowRightEllipsis) {
      // Show left ellipsis, no right ellipsis
      pages.push('ellipsis')
      for (let i = Math.max(totalPages - 4, 2); i < totalPages; i++) {
        pages.push(i)
      }
    } else if (shouldShowLeftEllipsis && shouldShowRightEllipsis) {
      // Show both ellipsis
      pages.push('ellipsis')
      for (let i = leftSiblingIndex; i <= rightSiblingIndex; i++) {
        pages.push(i)
      }
      pages.push('ellipsis')
    } else {
      // No ellipsis needed
      for (let i = 2; i < totalPages; i++) {
        pages.push(i)
      }
    }

    // Always show last page
    if (showFirstLast) {
      pages.push(totalPages)
    }

    return pages
  }, [currentPage, totalPages, siblingCount, showFirstLast])

  return (
    <Flex
      alignItems="center"
      className={clsx(
        css({
          gap: '4px',
        }),
        className,
      )}
      {...props}
    >
      {pageNumbers.map((page, index) =>
        page === 'ellipsis' ? (
          <PaginationEllipsis key={`ellipsis-${index}`} />
        ) : (
          <PaginationPageButton key={page} page={page} />
        ),
      )}
    </Flex>
  )
}

export {
  Pagination,
  PaginationContainer,
  PaginationEllipsis,
  PaginationNextButton,
  PaginationPageButton,
  PaginationPages,
  PaginationPrevButton,
}
