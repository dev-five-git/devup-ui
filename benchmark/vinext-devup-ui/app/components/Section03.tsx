import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 03. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section03() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #8b5cf6"
      borderRadius={11}
      boxShadow="0 3px 6px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={11}
      p={19}
      w="100%"
    >
      <Flex alignItems="center" gap={9} justifyContent="space-between" w="100%">
        <Text color="#8b5cf6" fontSize={19} fontWeight={700} lineHeight={1.3}>
          Section 03
        </Text>
        <Box
          bg="#14b8a6"
          borderRadius={999}
          color="#ffffff"
          fontSize={13}
          fontWeight={600}
          px={11}
          py={6}
        >
          Tag 03
        </Box>
      </Flex>
      <Box bg="#14b8a6" h={1} w="100%" />
      <Text color="#4b5563" fontSize={15} lineHeight={1.6}>
        Shared section 03 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={7} mt={5} w="100%">
        <Center
          _hover={{ bg: '#8b5cf6' }}
          bg="#f9fafb"
          borderRadius={7}
          color="#8b5cf6"
          cursor="pointer"
          flex={1}
          fontSize={14}
          px={13}
          py={11}
        >
          Primary 03
        </Center>
        <Center
          _hover={{ opacity: 0.3 }}
          bg="#14b8a6"
          borderRadius={7}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={14}
          px={13}
          py={11}
        >
          Secondary 03
        </Center>
      </Flex>
    </Flex>
  )
}
