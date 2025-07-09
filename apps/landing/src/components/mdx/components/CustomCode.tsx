import { Box } from '@devup-ui/react'

export function CustomCode({ children }: { children: string }) {
  return (
    <Box as="code" color="$primary" whiteSpace="pre-wrap">
      {children.replaceAll('<br>', '\n')}
    </Box>
  )
}
