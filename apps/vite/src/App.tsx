import { Box, Text } from '@devup-ui/react'
import { Lib } from 'vite-lib-example'

export default function App() {
  return (
    <div>
      <Box
        _hover={{
          bg: 'blue',
        }}
        bg="$text"
        color="red"
      >
        hello
        <Lib />
      </Box>
      <Text color="#777777"></Text>
      <Text color="#777"></Text>
      <Text color="#777"></Text>
      <Text typography="header">typo</Text>
    </div>
  )
}
