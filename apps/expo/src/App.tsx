import {
  Box,
  Button,
  Flex,
  Image,
  Input,
  setTheme,
  Text,
  useTheme,
} from '@devup-ui/react'
import { useState } from 'react'

import { Badge } from './Badge'

export function App() {
  const [name, setName] = useState('')
  const [count, setCount] = useState(0)
  const theme = useTheme()
  return (
    <Flex
      alignItems="center"
      bg="$background"
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
      <Text color="$text" typography="title">
        Devup UI on React Native
      </Text>
      <Badge label={`${theme ?? 'light'} theme`} />
      <Box bg="$card" borderRadius="$radius" boxShadow="$raised" p={4} w="100%">
        <Text color="$muted" fontSize="14px" mb={2}>
          Your name
        </Text>
        <Input
          border="1px solid $border"
          borderRadius="8px"
          color="$text"
          onChangeText={setName}
          px={3}
          py={2}
          value={name}
        />
        <Text color={name ? '$text' : '$placeholder'} mt={2}>
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
      <Button
        border="1px solid $border"
        borderRadius="8px"
        onPress={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
        px={5}
        py={3}
      >
        <Text color="$text">
          Switch to {theme === 'dark' ? 'light' : 'dark'}
        </Text>
      </Button>
    </Flex>
  )
}
