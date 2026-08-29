import { Box, Text } from '@devup-ui/react'

import { splitCodeLines } from './split-code-lines'

export function CustomCode({ children }: { children: string }) {
  return (
    <Box as="code" color="$primary" whiteSpace="pre-wrap">
      {splitCodeLines(children).map((line) => (
        <Text key={line.key} whiteSpace="pre">
          {line.startsNewLine && <br />}
          {line.text}
        </Text>
      ))}
    </Box>
  )
}
