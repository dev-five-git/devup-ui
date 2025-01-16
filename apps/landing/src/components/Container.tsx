import { Box } from '@devup-ui/react'

interface ContainerProps {
  children: React.ReactNode
}
export function Container({ children }: ContainerProps) {
  return (
    <Box maxW="1440px" mx="auto">
      {children}
    </Box>
  )
}
