import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const TableRow = ({ ...props }: ComponentPropsWithoutRef<'tr'>) => {
  return (
    <Box
      {...props}
      as="tr"
      borderBottom="1px solid var(--border, #E4E4E4)"
      selectors={{
        '& + &:last-of-type': {
          borderBottom: 'none',
        },
      }}
    />
  )
}
