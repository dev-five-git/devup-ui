'use client'
import { VStack } from '@devup-ui/react'
import * as bodyScrollLock from 'body-scroll-lock'

interface MobMenuWrapProps {
  children?: React.ReactNode
}

export function MobMenuWrap({ children }: MobMenuWrapProps) {
  return (
    <VStack
      ref={(ref) => {
        bodyScrollLock.disableBodyScroll(ref!)

        function ev() {
          if (window.innerWidth >= 768) bodyScrollLock.enableBodyScroll(ref!)
        }

        window.addEventListener('resize', ev)
        return () => {
          bodyScrollLock.enableBodyScroll(ref!)
          window.removeEventListener('resize', ev)
        }
      }}
      bg="$containerBackground"
      borderTop="1px solid var(--border, #E0E0E0)"
      h="100%"
      left={0}
      maxH="calc(100vh - 50px)"
      overflowY="auto"
      pos="fixed"
      top="50px"
      w="100%"
      zIndex={100}
    >
      {children}
    </VStack>
  )
}
