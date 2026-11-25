import { Box, Button, Flex, Image, Input, Text } from '@devup-ui/react'
import { useState } from 'react'

export function App() {
  const [name, setName] = useState('')
  const [count, setCount] = useState(0)
  return (
    <Flex
      alignItems="center"
      bg="#f6f7fb"
      flex={1}
      flexDirection="column"
      gap={4}
      justifyContent="center"
      p={6}
    >
      <Image
        borderRadius="16px"
        h="64px"
        src="https://reactnative.dev/img/tiny_logo.png"
        w="64px"
      />
      <Text color="#1a1a2e" fontSize="24px" fontWeight="700">
        Devup UI on React Native
      </Text>
      <Box
        bg="#fff"
        borderRadius="12px"
        boxShadow="0 2px 8px #0002"
        p={4}
        w="100%"
      >
        <Text color="#555" fontSize="14px" mb={2}>
          Your name
        </Text>
        <Input
          border="1px solid #ccc"
          borderRadius="8px"
          onChangeText={setName}
          px={3}
          py={2}
          value={name}
        />
        <Text color={name ? '#1a1a2e' : '#999'} mt={2}>
          {name ? `Hello, ${name}` : 'Type your name'}
        </Text>
      </Box>
      <Button
        bg="#5a44ff"
        borderRadius="8px"
        onPress={() => setCount(count + 1)}
        px={5}
        py={3}
      >
        <Text color="#fff" fontWeight="600">
          Pressed {count} times
        </Text>
      </Button>
    </Flex>
  )
}
