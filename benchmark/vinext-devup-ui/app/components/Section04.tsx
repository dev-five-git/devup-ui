import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 04. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section04() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #ef4444"
      borderRadius={12}
      boxShadow="0 4px 8px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={12}
      p={20}
      w="100%"
    >
      <Flex
        alignItems="center"
        gap={10}
        justifyContent="space-between"
        w="100%"
      >
        <Text color="#ef4444" fontSize={20} fontWeight={700} lineHeight={1.3}>
          Section 04
        </Text>
        <Box
          bg="#d946ef"
          borderRadius={999}
          color="#ffffff"
          fontSize={14}
          fontWeight={600}
          px={12}
          py={7}
        >
          Tag 04
        </Box>
      </Flex>
      <Box bg="#d946ef" h={2} w="100%" />
      <Text color="#4b5563" fontSize={16} lineHeight={1.6}>
        Shared section 04 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={8} mt={6} w="100%">
        <Center
          _hover={{ bg: '#ef4444' }}
          bg="#f9fafb"
          borderRadius={8}
          color="#ef4444"
          cursor="pointer"
          flex={1}
          fontSize={15}
          px={14}
          py={12}
        >
          Primary 04
        </Center>
        <Center
          _hover={{ opacity: 0.4 }}
          bg="#d946ef"
          borderRadius={8}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={15}
          px={14}
          py={12}
        >
          Secondary 04
        </Center>
      </Flex>
    </Flex>
  )
}
