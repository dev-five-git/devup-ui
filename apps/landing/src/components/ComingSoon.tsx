import { Box, Text, VStack } from '@devup-ui/react'

export const ComingSoon = () => {
  return (
    <Box
      bg="$cardBg"
      borderRadius="12px"
      p="40px"
      textAlign="center"
      width="100%"
    >
      <VStack alignItems="center" gap="12px">
        <Text color="$textSecondary" fontSize="32px">
          🚧
        </Text>
        <Text color="$title" typography="h6">
          Coming Soon
        </Text>
        <Text color="$textSecondary" typography="body">
          This section is currently under construction.
        </Text>
      </VStack>
    </Box>
  )
}
