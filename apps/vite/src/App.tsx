import { Box, Center, Text } from '@devup-ui/react'
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
      <NoContent />
    </div>
  )
}
export function NoContent(props: { text?: string; background?: boolean }) {
  const { text, background } = props
  return (
    <Center
      bg={background ? 'var(--white-bg)' : undefined}
      h="100%"
      minH="80px"
      w="100%"
    >
      <Text color="var(--gray-color)" fontSize="13px" textAlign="center">
        {text ?? '표시할 정보가 없습니다.'}
      </Text>
    </Center>
  )
}
