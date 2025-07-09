import { Box, Text } from '@devup-ui/react'

export function CustomCode({ children }: { children: string }) {
  return (
    <Box as="code" color="$primary" whiteSpace="pre-wrap">
      {children.split('<br>').map((line, index) => (
        <Text key={index} whiteSpace="pre">
          {index > 0 && <br />}
          {line}
        </Text>
      ))}
    </Box>
  )
}
