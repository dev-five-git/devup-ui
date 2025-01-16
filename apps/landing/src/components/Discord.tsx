import { Center, Flex, Image, Text, VStack } from '@devup-ui/react'

import { IMAGE_PREFIX } from '../constants'

export function Discord() {
  return (
    <Center
      bgColor="$joinBg"
      bgImage="url(/discord-bg.svg)"
      borderRadius="40px 40px 0px 40px"
      h="380px"
    >
      <VStack alignItems="flex-end" gap="50px" ml="auto" pr="100px" w="548px">
        <VStack gap="16px">
          <Text color="$title" typography="h4">
            Join our community
          </Text>
          <Text typography="textL">
            Etiam sit amet feugiat turpis. Proin nec ante a sem vestibulum
            sodales non ut ex. Morbi diam turpis, fringilla vitae enim et,
            egestas consequat nibh.
          </Text>
        </VStack>
        <Flex
          alignItems="center"
          bg="$buttonBlue"
          borderRadius="100px"
          p="16px 40px"
        >
          <Flex alignItems="center" gap="10px">
            <Text color="#FFF" typography="buttonLbold">
              Join our Discord
            </Text>
            <Image boxSize="24px" src={IMAGE_PREFIX + '/outlink.svg'} />
          </Flex>
        </Flex>
      </VStack>
    </Center>
  )
}
