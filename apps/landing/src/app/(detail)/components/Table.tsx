import { Box } from '@devup-ui/react'

export function Table(props: React.ComponentProps<'table'>) {
  return (
    <Box
      as="table"
      border="none"
      selectors={{
        '& th, & td': {
          border: 'none',
          minWidth: '200px',
        },
      }}
      styleOrder={1}
      {...props}
    />
  )
}

export function Tr(props: React.ComponentProps<'tr'>) {
  return (
    <Box
      as="tr"
      borderBottom="1px solid $border"
      borderTop="1px solid $border"
      {...props}
    />
  )
}

export function Td(props: React.ComponentProps<'td'>) {
  return (
    <Box
      as="td"
      border="none"
      px="20px"
      py="14px"
      styleOrder={1}
      width="fit-content"
      {...props}
    />
  )
}

export function Th(props: React.ComponentProps<'th'>) {
  return (
    <Box
      as="th"
      bg="$cardBg"
      border="none"
      borderBottom="1px solid $border"
      borderTop="1px solid $border"
      color="$captionBold"
      px="20px"
      py="14px"
      styleOrder={1}
      textAlign="left"
      typography="bodyBold"
      {...props}
    />
  )
}
