import { Box, Text } from '@devup-ui/react'
import { Lib } from 'vite-lib-example'

import { VanillaExtractExample } from './VanillaExtractExample'

export default function App() {
  return (
    <div>
      <VanillaExtractExample />
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
      <Text fontWeight="bold">wf</Text>
      <Text color="#777"></Text>
      <Text color="#777"></Text>
      <Text typography="header">typo</Text>
    </div>
  )
}
