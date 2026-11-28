'use client'
import { Box, Text, VStack } from '@devup-ui/react'
import { useTranslations } from 'next-intl'

export function ComingSoon() {
  const t = useTranslations('ComingSoon')
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
          {t('title')}
        </Text>
        <Text color="$textSecondary" typography="body">
          {t('description')}
        </Text>
      </VStack>
    </Box>
  )
}
