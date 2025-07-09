import { Text } from '@devup-ui/react'

export function CustomH4({ children }: { children: React.ReactNode }) {
  return (
    <Text as="h4" color="$title" m="0" typography="h4">
      {children}
    </Text>
  )
}
