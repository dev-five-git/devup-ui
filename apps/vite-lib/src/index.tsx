import { Box, css, Flex, Text } from '@devup-ui/react'
const a = css``

export function Lib() {
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
      <Flex as="section" mt={2}>
        section
      </Flex>
    </div>
  )
}
