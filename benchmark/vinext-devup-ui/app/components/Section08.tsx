import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 08. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section08() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #ef4444"
      borderRadius={16}
      boxShadow="0 8px 16px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={16}
      p={24}
      w="100%"
    >
      <Flex
        alignItems="center"
        gap={14}
        justifyContent="space-between"
        w="100%"
      >
        <Text color="#ef4444" fontSize={24} fontWeight={700} lineHeight={1.3}>
          Section 08
        </Text>
        <Box
          bg="#06b6d4"
          borderRadius={999}
          color="#ffffff"
          fontSize={18}
          fontWeight={600}
          px={16}
          py={11}
        >
          Tag 08
        </Box>
      </Flex>
      <Box bg="#06b6d4" h={3} w="100%" />
      <Text color="#4b5563" fontSize={20} lineHeight={1.6}>
        Shared section 08 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={12} mt={10} w="100%">
        <Center
          _hover={{ bg: '#ef4444' }}
          bg="#f9fafb"
          borderRadius={12}
          color="#ef4444"
          cursor="pointer"
          flex={1}
          fontSize={19}
          px={18}
          py={16}
        >
          Primary 08
        </Center>
        <Center
          _hover={{ opacity: 0.8 }}
          bg="#06b6d4"
          borderRadius={12}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={19}
          px={18}
          py={16}
        >
          Secondary 08
        </Center>
      </Flex>
    </Flex>
  )
}
