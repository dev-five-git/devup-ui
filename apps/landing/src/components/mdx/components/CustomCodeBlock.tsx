import { Box, Text } from '@devup-ui/react'

import { splitCodeLines } from './split-code-lines'

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
      {splitCodeLines(children).map((line) => (
        <Text key={line.key} whiteSpace="pre">
          {line.startsNewLine && <br />}
          {line.text}
        </Text>
      ))}
    </Box>
  )
}
