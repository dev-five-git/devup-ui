// The components typed for React Native (`@devup-ui/react/native/types`):
// each takes the props of the React Native component it renders, and its
// style props as on the web
import { Box, Button, css, Flex, Image, Input, Text } from '@devup-ui/react'

export const app = (
  <Box bg="red" className={css({ p: 1 })} onLayout={() => {}} testID="card">
    <Flex accessibilityRole="header" gap={2}>
      <Text numberOfLines={1} onPress={() => {}}>
        title
      </Text>
    </Flex>
    <Input
      _focus={{ borderColor: 'blue' }}
      onChangeText={(text: string) => text}
      secureTextEntry
    />
    <Button _active={{ opacity: 0.5 }} onPress={() => {}}>
      <Text>press</Text>
    </Button>
    <Image h={10} source={{ uri: 'https://example.com/a.png' }} w={10} />
  </Box>
)

// @ts-expect-error a View has no href
export const link = <Box href="/" />
// @ts-expect-error a TextInput reports its text with onChangeText
export const input = <Input onInput={() => {}} />
