import { Text } from '@devup-ui/react'

export function CustomStrong({ children }: { children: React.ReactNode }) {
  return (
    <Text as="strong" color="$primary" m="0" typography="captionBold">
      {children}
    </Text>
  )
}
