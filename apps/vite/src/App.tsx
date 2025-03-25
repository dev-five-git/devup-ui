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
      <Text typography="header">typo</Text>
    </div>
  )
}
