import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const Table = ({ ...props }: ComponentPropsWithoutRef<'table'>) => {
  return (
    <Box borderRadius="0.5rem" overflow="hidden">
      <Box as="table" borderCollapse="collapse" borderSpacing={0} {...props} />
    </Box>
  )
}

export const Table = ({ ...props }: ComponentPropsWithoutRef<'thead'>) => {
  return (
    <Box
      as="thead"
      {...props}
      selectors={{
        '& tr': {
          bg: '$cardBg',
        },
      }}
    />
  )
}

export const Table = ({ ...props }: ComponentPropsWithoutRef<'tbody'>) => {
  return <Box as="tbody" {...props}></Box>
}

export const Table = ({ ...props }: ComponentPropsWithoutRef<'tr'>) => {
  return (
    <Box
      as="tr"
      borderBottom="1px solid var(--border, #E4E4E4)"
      selectors={{
        '& + &:last-of-type': {
          borderBottom: 'none',
        },
      }}
      {...props}
    />
  )
}

export const Table = ({ ...props }: ComponentPropsWithoutRef<'td'>) => {
  return <Box as="td" padding="0.5rem 1rem" {...props} />
}

export const Table = ({ ...props }: ComponentPropsWithoutRef<'th'>) => {
  return <Box as="th" padding="0.5rem 1rem" textAlign="left" {...props} />
}
