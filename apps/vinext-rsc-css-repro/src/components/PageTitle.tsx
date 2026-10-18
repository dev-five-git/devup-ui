import { Box, Flex, Text } from '@devup-ui/react'
import type { ReactNode } from 'react'

export function PageTitle({
  children,
  right,
}: {
  children: ReactNode
  right?: ReactNode
}) {
  return (
    <Flex
      alignItems="center"
      data-page-title=""
      justifyContent="space-between"
      mb="20px"
    >
      <Box alignItems="center" display="flex" gap={3}>
        <button type="button">Back</button>
        <Text>{children}</Text>
      </Box>
      {right}
    </Flex>
  )
}
