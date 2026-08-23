import { Box, Center, Flex, Text } from '@devup-ui/react'

/**
 * Shared section 07. Imported by an overlapping subset of the routes, so its
 * atoms must land in one shared CSS chunk instead of being copied into every
 * route chunk that uses it.
 */
export function Section07() {
  return (
    <Flex
      alignItems="stretch"
      bg="#ffffff"
      border="1px solid #8b5cf6"
      borderRadius={15}
      boxShadow="0 7px 14px rgba(0,0,0,0.08)"
      flexDirection="column"
      gap={15}
      p={23}
      w="100%"
    >
      <Flex
        alignItems="center"
        gap={13}
        justifyContent="space-between"
        w="100%"
      >
        <Text color="#8b5cf6" fontSize={23} fontWeight={700} lineHeight={1.3}>
          Section 07
        </Text>
        <Box
          bg="#f97316"
          borderRadius={999}
          color="#ffffff"
          fontSize={17}
          fontWeight={600}
          px={15}
          py={10}
        >
          Tag 07
        </Box>
      </Flex>
      <Box bg="#f97316" h={2} w="100%" />
      <Text color="#4b5563" fontSize={19} lineHeight={1.6}>
        Shared section 07 body copy.
      </Text>
      <Flex flexWrap="wrap" gap={11} mt={9} w="100%">
        <Center
          _hover={{ bg: '#8b5cf6' }}
          bg="#f9fafb"
          borderRadius={11}
          color="#8b5cf6"
          cursor="pointer"
          flex={1}
          fontSize={18}
          px={17}
          py={15}
        >
          Primary 07
        </Center>
        <Center
          _hover={{ opacity: 0.7 }}
          bg="#f97316"
          borderRadius={11}
          color="#ffffff"
          cursor="pointer"
          flex={1}
          fontSize={18}
          px={17}
          py={15}
        >
          Secondary 07
        </Center>
      </Flex>
    </Flex>
  )
}
