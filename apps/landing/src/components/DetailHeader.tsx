import { Flex, Image, Text, VStack } from '@devup-ui/react'

export function DetailHeader() {
  return (
    <VStack
      alignItems="center"
      bg="$background"
      borderColor="$border"
      boxShadow="0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))"
      pos="absolute"
      px="240px"
      w="1920px"
    >
      <Flex
        alignItems="center"
        borderRadius="16px"
        h="70px"
        justifyContent="space-between"
        px="40px"
      >
        <Flex alignItems="center" gap="16px">
          <Image h="24.27px" w="25.475px" />
        </Flex>
        <Flex alignItems="center" gap="10px">
          <Flex alignItems="center" px="24px">
            <Text color="$primary" typography="buttonLbold">
              Docs
            </Text>
          </Flex>
          <Flex alignItems="center" px="24px">
            <Text color="$title" opacity="0.6" typography="buttonLsemiB">
              Team
            </Text>
          </Flex>
          <Flex
            alignItems="center"
            bg="$menuHover"
            borderRadius="8px"
            gap="10px"
            p="8px 8px 8px 6px"
            w="240px"
          >
            <Image boxSize="24px" opacity="0.6" src="icons" />
            <Text color="$caption" typography="caption">
              Search documentation...
            </Text>
          </Flex>
          <Flex alignItems="center">
            <Flex alignItems="center" px="10px">
              <Image boxSize="24px" src="gnb icon" />
            </Flex>
            <Flex alignItems="center" px="10px">
              <Image boxSize="24px" src="gnb icon" />
            </Flex>
            <Flex alignItems="center" px="10px">
              <Image boxSize="24px" src="gnb icon" />
            </Flex>
          </Flex>
        </Flex>
      </Flex>
    </VStack>
  )
}
