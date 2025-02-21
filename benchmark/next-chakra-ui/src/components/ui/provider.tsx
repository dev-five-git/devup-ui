'use client'

import { ChakraProvider, defaultSystem } from '@chakra-ui/react'

import { ColorMode, type ColorModeProviderProps } from './color-mode'

export function Provider(props: ColorModeProviderProps) {
  return (
    <ChakraProvider value={defaultSystem}>
      <ColorMode {...props} />
    </ChakraProvider>
  )
}
