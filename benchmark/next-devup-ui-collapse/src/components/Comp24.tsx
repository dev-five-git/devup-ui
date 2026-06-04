import { Box, Center, Flex, Text } from '@devup-ui/react'

export function Comp24() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #e5e7eb"
      borderRadius={12}
      boxShadow="0 1px 3px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={12}
      p={20}
      w="100%"
    >
      <Flex alignItems="center" justifyContent="space-between" w="100%">
        <Text color="#111827" fontSize={18} fontWeight={700} lineHeight={1.3}>
          Card 24
        </Text>
        <Box
          bg="#ec4899"
          borderRadius={999}
          color="#ffffff"
          fontSize={12}
          fontWeight={600}
          px={10}
          py={4}
        >
          Label
        </Box>
      </Flex>
      <Box bg="#f3f4f6" h={1} w="100%" />
      <Text color="#6b7280" fontSize={14} lineHeight={1.6}>
        Reusable card with shared atomic styles across single-importer
        components.
      </Text>
      <Flex flexWrap="wrap" gap={8} mt={4} w="100%">
        <Center
          _hover={{ bg: '#f3f4f6' }}
          bg="#f9fafb"
          borderRadius={8}
          color="#374151"
          cursor="pointer"
          flex={1}
          fontSize={13}
          px={12}
          py={10}
        >
          Action A
        </Center>
        <Center
          _hover={{ opacity: 0.9 }}
          bg="#ec4899"
          borderRadius={8}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={13}
          px={12}
          py={10}
        >
          Action B
        </Center>
      </Flex>
    </Flex>
  )
}
