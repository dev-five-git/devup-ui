import { Text } from '@devup-ui/react'
import type { ReactNode } from 'react'

export function LandingTitle({ children }: { children: ReactNode }) {
  return (
    <Text as="h1" color="$title" m="0" px={[6, null, 0]} typography="h1">
      {children}
    </Text>
  )
}
