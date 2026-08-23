import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 06. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section06() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #06b6d4"
      borderRadius={14}
      boxShadow="0 6px 12px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={14}
      p={22}
      w="100%"
    >
      <Flex
        alignItems="center"
        gap={12}
        justifyContent="space-between"
        w="100%"
      >
        <Text color="#06b6d4" fontSize={22} fontWeight={700} lineHeight={1.3}>
          Section 06
        </Text>
        <Box
          bg="#6366f1"
          borderRadius={999}
          color="#ffffff"
          fontSize={16}
          fontWeight={600}
          px={14}
          py={9}
        >
          Tag 06
        </Box>
      </Flex>
      <Box bg="#6366f1" h={1} w="100%" />
      <Text color="#4b5563" fontSize={18} lineHeight={1.6}>
        Shared section 06 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={10} mt={8} w="100%">
        <Center
          _hover={{ bg: '#06b6d4' }}
          bg="#f9fafb"
          borderRadius={10}
          color="#06b6d4"
          cursor="pointer"
          flex={1}
          fontSize={17}
          px={16}
          py={14}
        >
          Primary 06
        </Center>
        <Center
          _hover={{ opacity: 0.6 }}
          bg="#6366f1"
          borderRadius={10}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={17}
          px={16}
          py={14}
        >
          Secondary 06
        </Center>
      </Flex>
    </Flex>
  )
}
