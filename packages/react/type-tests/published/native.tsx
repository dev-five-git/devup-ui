// Type-checks a React Native project against the published declarations: the
// import `@devup-ui/metro-plugin` writes types each component as the React
// Native component it renders
import '@devup-ui/react/native/types'

import { Box, Button, Input } from '@devup-ui/react'

export const app = (
  <Box onLayout={() => {}} p={2}>
    <Input onChangeText={(text: string) => text} />
    <Button onPress={() => {}} />
  </Box>
)

// @ts-expect-error a View has no href
export const link = <Box href="/" />
