import { Box, css } from '@devup-ui/react'
import clsx from 'clsx'

export function Table({
  children,
  className,
  ...props
}: React.ComponentProps<'table'>) {
  return (
    <Box
      as="table"
      className={clsx(
        css({
          border: 'none',
          styleOrder: 1,
        }),
        className,
      )}
      selectors={{
        '& th, & td': {
          border: 'none',
        },
      }}
      {...props}
    >
      {children}
    </Box>
  )
}

export function Tr({ children, ...props }: React.ComponentProps<'tr'>) {
  return (
    <Box
      as="tr"
      className={css({
        borderTop: '1px solid $border',
        borderBottom: '1px solid $border',
      })}
      {...props}
    >
      {children}
    </Box>
  )
}

export function Td({
  children,
  className,
  ...props
}: React.ComponentProps<'td'>) {
  return (
    <Box
      as="td"
      className={clsx(
        css({
          border: 'none',
          py: '14px',
          px: '20px',
          width: 'fit-content',
          styleOrder: 1,
        }),
        className,
      )}
      {...props}
    >
      {children}
    </Box>
  )
}

export function Th({
  children,
  className,
  ...props
}: React.ComponentProps<'th'>) {
  return (
    <Box
      as="th"
      border="none"
      className={clsx(
        css({
          py: '14px',
          px: '20px',
          color: '$captionBold',
          typography: 'bodyBold',
          borderTop: '1px solid $border',
          borderBottom: '1px solid $border',
          bg: '$cardBg',
          textAlign: 'left',
          styleOrder: 1,
        }),
        className,
      )}
      {...props}
    >
      {children}
    </Box>
  )
}
