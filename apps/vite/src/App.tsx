import { Box, Text } from '@devup-ui/react'

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
      </Box>
      <Text typography="header">typo</Text>
    </div>
  )
}
