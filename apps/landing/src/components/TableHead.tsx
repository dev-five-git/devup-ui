import { Box } from '@devup-ui/react'
import { ComponentPropsWithoutRef } from 'react'

export const TableHead = ({ ...props }: ComponentPropsWithoutRef<'thead'>) => {
  return (
    <Box
      {...props}
      as="thead"
      selectors={{
        '& tr': {
          bg: '$cardBg',
        },
      }}
    />
  )
}
