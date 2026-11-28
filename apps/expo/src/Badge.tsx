import { Text } from '@devup-ui/react'

/** Styles the same in every theme, so it compiles without the runtime */
export function Badge({ label }: { label: string }) {
  return (
    <Text
      bg="#5a44ff"
      borderRadius="999px"
      color="#fff"
      fontSize="12px"
      fontWeight="600"
      px={2}
      py={1}
    >
      {label}
    </Text>
  )
}
