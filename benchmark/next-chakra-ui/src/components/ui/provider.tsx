'use client'

import { ChakraProvider, createSystem, defaultConfig } from '@chakra-ui/react'

import { ColorMode, type ColorModeProviderProps } from './color-mode'

// No css reset, like the other benchmark fixtures
const system = createSystem(defaultConfig, { preflight: false })

export function Provider(props: ColorModeProviderProps) {
  return (
    <ChakraProvider value={system}>
      <ColorMode {...props} />
    </ChakraProvider>
  )
}
