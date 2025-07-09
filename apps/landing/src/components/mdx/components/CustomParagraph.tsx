import { Text } from '@devup-ui/react'

export function CustomParagraph({ children }: { children: React.ReactNode }) {
  return (
    <Text as="p" color="$text" lineHeight="1" m="0" typography="bodyReg">
      {children}
    </Text>
  )
}
