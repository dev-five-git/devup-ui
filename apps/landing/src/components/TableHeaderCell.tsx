import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const TableHeaderCell = ({
  ...props
}: ComponentPropsWithoutRef<'th'>) => {
  return <Box {...props} as="th" padding="0.5rem 1rem" textAlign="left" />
}
