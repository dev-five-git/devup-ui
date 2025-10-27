import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const TableBody = ({ ...props }: ComponentPropsWithoutRef<'tbody'>) => {
  return <Box {...props} as="tbody" />
}
