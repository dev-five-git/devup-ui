import { Box, Text } from '@devup-ui/react'
import { Lib } from 'vite-lib-example'

import { Button } from './Button'

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
      <Button size="s" variant="primary">
        hello
      </Button>
    </div>
  )
}
