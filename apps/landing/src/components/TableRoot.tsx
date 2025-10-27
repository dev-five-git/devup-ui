import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const TableRoot = ({ ...props }: ComponentPropsWithoutRef<'table'>) => {
  return (
    <Box borderRadius="0.5rem" overflow="hidden">
      <Box {...props} as="table" borderCollapse="collapse" borderSpacing={0} />
    </Box>
  )
}
