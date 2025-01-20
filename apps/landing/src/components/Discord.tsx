import { Center, css, Flex, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

import { URL_PREFIX } from '../constants'

export function Discord() {
  return (
    <Center
      bgColor="$joinBg"
      bgImage={`url(${URL_PREFIX}/discord-bg.svg)`}
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
        <Link
          className={css`
            text-decoration: none;
          `}
          href="https://discord.gg/BtNffusw"
          target="_blank"
        >
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
              <Image boxSize="24px" src={URL_PREFIX + '/outlink.svg'} />
            </Flex>
          </Flex>
        </Link>
      </VStack>
    </Center>
  )
}
