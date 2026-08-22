import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 05. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section05() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #84cc16"
      borderRadius={13}
      boxShadow="0 5px 10px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={13}
      p={21}
      w="100%"
    >
      <Flex
        alignItems="center"
        gap={11}
        justifyContent="space-between"
        w="100%"
      >
        <Text color="#84cc16" fontSize={21} fontWeight={700} lineHeight={1.3}>
          Section 05
        </Text>
        <Box
          bg="#84cc16"
          borderRadius={999}
          color="#ffffff"
          fontSize={15}
          fontWeight={600}
          px={13}
          py={8}
        >
          Tag 05
        </Box>
      </Flex>
      <Box bg="#84cc16" h={3} w="100%" />
      <Text color="#4b5563" fontSize={17} lineHeight={1.6}>
        Shared section 05 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={9} mt={7} w="100%">
        <Center
          _hover={{ bg: '#84cc16' }}
          bg="#f9fafb"
          borderRadius={9}
          color="#84cc16"
          cursor="pointer"
          flex={1}
          fontSize={16}
          px={15}
          py={13}
        >
          Primary 05
        </Center>
        <Center
          _hover={{ opacity: 0.5 }}
          bg="#84cc16"
          borderRadius={9}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={16}
          px={15}
          py={13}
        >
          Secondary 05
        </Center>
      </Flex>
    </Flex>
  )
}
