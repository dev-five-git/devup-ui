import { Text } from '@devup-ui/react'

export function CustomH6({ children }: { children: React.ReactNode }) {
  return (
    <Text as="h6" color="$title" m="0" typography="h6">
      {children}
    </Text>
  )
}
