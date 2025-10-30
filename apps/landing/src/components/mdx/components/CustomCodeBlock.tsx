import { Box, Text } from '@devup-ui/react'

export function CustomCodeBlock({ children }: { children: string }) {
  return (
    <Box
      as="code"
      bg="$codeBackground"
      borderRadius="0.25rem"
      color="$text"
      padding="0.25rem"
      whiteSpace="pre-wrap"
    >
      {children.split('<br>').map((line, index) => (
        <Text key={index.toString()} whiteSpace="pre">
          {index > 0 && <br />}
          {line}
        </Text>
      ))}
    </Box>
  )
}
