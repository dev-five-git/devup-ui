import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 01. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section01() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #84cc16"
      borderRadius={9}
      boxShadow="0 1px 2px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={9}
      p={17}
      w="100%"
    >
      <Flex alignItems="center" gap={7} justifyContent="space-between" w="100%">
        <Text color="#84cc16" fontSize={17} fontWeight={700} lineHeight={1.3}>
          Section 01
        </Text>
        <Box
          bg="#3b82f6"
          borderRadius={999}
          color="#ffffff"
          fontSize={11}
          fontWeight={600}
          px={9}
          py={4}
        >
          Tag 01
        </Box>
      </Flex>
      <Box bg="#3b82f6" h={2} w="100%" />
      <Text color="#4b5563" fontSize={13} lineHeight={1.6}>
        Shared section 01 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={5} mt={3} w="100%">
        <Center
          _hover={{ bg: '#84cc16' }}
          bg="#f9fafb"
          borderRadius={5}
          color="#84cc16"
          cursor="pointer"
          flex={1}
          fontSize={12}
          px={11}
          py={9}
        >
          Primary 01
        </Center>
        <Center
          _hover={{ opacity: 0.1 }}
          bg="#3b82f6"
          borderRadius={5}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={12}
          px={11}
          py={9}
        >
          Secondary 01
        </Center>
      </Flex>
    </Flex>
  )
}
