'use client'

import type { IconButtonProps } from '@chakra-ui/react'
import { ClientOnly, IconButton, Skeleton } from '@chakra-ui/react'
import type { ThemeProviderProps } from 'next-themes'
import { ThemeProvider, useTheme } from 'next-themes'
import type { Ref } from 'react'
import { LuMoon, LuSun } from 'react-icons/lu'

export type ColorModeProviderProps = ThemeProviderProps

export function ColorMode(props: ColorModeProviderProps) {
  return (
    <ThemeProvider attribute="class" disableTransitionOnChange {...props} />
  )
}

export function useColorMode() {
  const { resolvedTheme, setTheme } = useTheme()
  const toggleColorMode = () => {
    setTheme(resolvedTheme === 'light' ? 'dark' : 'light')
  }
  return {
    colorMode: resolvedTheme,
    setColorMode: setTheme,
    toggleColorMode,
  }
}

export function ColorModeIcon() {
  const { colorMode } = useColorMode()
  return colorMode === 'light' ? <LuSun /> : <LuMoon />
}

type ColorModeButtonProps = Omit<IconButtonProps, 'aria-label'> & {
  ref?: Ref<HTMLButtonElement>
}

export function ColorModeButton({ ref, ...props }: ColorModeButtonProps) {
  const { toggleColorMode } = useColorMode()
  return (
    <ClientOnly fallback={<Skeleton boxSize="8" />}>
      <IconButton
        ref={ref}
        aria-label="Toggle color mode"
        onClick={toggleColorMode}
        size="sm"
        variant="ghost"
        {...props}
        css={{
          _icon: {
            width: '5',
            height: '5',
          },
        }}
      >
        <ColorModeIcon />
      </IconButton>
    </ClientOnly>
  )
}
