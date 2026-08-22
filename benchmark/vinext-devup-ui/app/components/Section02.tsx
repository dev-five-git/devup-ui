import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 02. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section02() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #06b6d4"
      borderRadius={10}
      boxShadow="0 2px 4px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={10}
      p={18}
      w="100%"
    >
      <Flex alignItems="center" gap={8} justifyContent="space-between" w="100%">
        <Text color="#06b6d4" fontSize={18} fontWeight={700} lineHeight={1.3}>
          Section 02
        </Text>
        <Box
          bg="#ef4444"
          borderRadius={999}
          color="#ffffff"
          fontSize={12}
          fontWeight={600}
          px={10}
          py={5}
        >
          Tag 02
        </Box>
      </Flex>
      <Box bg="#ef4444" h={3} w="100%" />
      <Text color="#4b5563" fontSize={14} lineHeight={1.6}>
        Shared section 02 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={6} mt={4} w="100%">
        <Center
          _hover={{ bg: '#06b6d4' }}
          bg="#f9fafb"
          borderRadius={6}
          color="#06b6d4"
          cursor="pointer"
          flex={1}
          fontSize={13}
          px={12}
          py={10}
        >
          Primary 02
        </Center>
        <Center
          _hover={{ opacity: 0.2 }}
          bg="#ef4444"
          borderRadius={6}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={13}
          px={12}
          py={10}
        >
          Secondary 02
        </Center>
      </Flex>
    </Flex>
  )
}
