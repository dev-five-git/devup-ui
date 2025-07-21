import { Box } from '@devup-ui/react'

export function CustomPre({ children }: { children: React.ReactNode }) {
  return (
    <Box
      as="pre"
      bg="transparent"
      lineBreak="anywhere"
      m="0"
      overflow="auto"
      selectors={{
        '& pre, & code, & span, & p': {
          margin: '0',
          w: '100%',
          whiteSpace: 'pre-wrap',
          lineBreak: 'anywhere',
          bg: 'transparent',
          overflow: 'auto',
        },
      }}
      w="100%"
      whiteSpace="pre-wrap"
    >
      {children}
    </Box>
  )
}
