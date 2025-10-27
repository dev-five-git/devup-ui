import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const TableCell = ({ ...props }: ComponentPropsWithoutRef<'th'>) => {
  return <Box {...props} as="td" padding="0.5rem 1rem" />
}
