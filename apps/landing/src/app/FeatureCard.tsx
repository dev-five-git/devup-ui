import { Flex, Image, Text, VStack } from '@devup-ui/react'

import { URL_PREFIX } from '../constants'

interface FeatureCardProps {
  title: string
  description: string
  icon: string
}

export function FeatureCard({ icon, description, title }: FeatureCardProps) {
  return (
    <Flex bg="$cardBg" borderRadius="20px" flex="1" gap="10px" p="24px">
      <Flex px="8px">
        <Image boxSize="32px" src={URL_PREFIX + icon} />
      </Flex>
      <VStack flex="1" gap="10px">
        <Text color="$title" typography="h6">
          {title}
        </Text>
        <Text color="$text" flex="1" typography="body">
          {description}
        </Text>
      </VStack>
    </Flex>
  )
}
