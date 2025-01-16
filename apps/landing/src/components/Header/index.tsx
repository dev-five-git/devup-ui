import { Box, Flex, Image, Text } from '@devup-ui/react'

import { Container } from '../Container'

export function Header() {
  return (
    <Box position="fixed" top={5} w="100%">
      <Container>
        <Flex
          alignItems="center"
          bg="$containerBackground"
          borderRadius="16px"
          boxShadow="0px 2px 8px 0px var(--shadow, rgba(135, 135, 135, 0.25))"
          h="70px"
          justifyContent="space-between"
          px="40px"
        >
          <Flex alignItems="center" gap="16px">
            <Image h="42px" src="/logo.svg" />
          </Flex>
          <Flex alignItems="center" gap="10px">
            <Flex alignItems="center" px="24px">
              <Text color="$title" opacity="0.6" typography="buttonLsemiB">
                Docs
              </Text>
            </Flex>
            <Flex alignItems="center" px="24px">
              <Text color="$title" opacity="0.6" typography="buttonLsemiB">
                Team
              </Text>
            </Flex>
            <Flex alignItems="center">
              <Flex alignItems="center" px="10px">
                <Image boxSize="24px" src="/github.svg" />
              </Flex>
              <Flex alignItems="center" px="10px">
                <Image boxSize="24px" src="/discord.svg" />
              </Flex>
              <Flex alignItems="center" px="10px">
                <Image boxSize="24px" src="/light.svg" />
              </Flex>
            </Flex>
          </Flex>
        </Flex>
      </Container>
    </Box>
  )
}
