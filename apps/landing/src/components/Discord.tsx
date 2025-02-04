import { css, Flex, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'

import { URL_PREFIX } from '../constants'

export function Discord() {
  return (
    <VStack
      alignItems="center"
      bgColor="$joinBg"
      bgImage={`url(${URL_PREFIX}/discord-bg.svg)`}
      bgPositionX={['-20vw', null, '-30%']}
      bgPositionY="bottom"
      bgSize={['contain', null, '70%']}
      borderRadius="40px 40px 0px 40px"
      h="380px"
      justifyContent={[null, null, 'center']}
      pt={[10, null, 0]}
    >
      <VStack
        alignItems={['center', null, 'flex-end']}
        gap="50px"
        ml={[null, null, 'auto']}
        pr={[null, null, '100px']}
      >
        <VStack gap="16px" px={5} textAlign={['center', null, 'left']}>
          <Text color="$title" typography="h4">
            Join our community
          </Text>
          <Text color="$text" typography="textL">
            Join our Discord and help build the future of frontend with
            CSS-in-JS!
          </Text>
        </VStack>
        <Link
          className={css({
            textDecoration: 'none',
            borderRadius: '100px',
          })}
          href="https://discord.gg/BtNffusw"
          target="_blank"
        >
          <Flex
            _active={{
              bg: '$buttonBlueActive',
            }}
            _hover={{
              bg: '$buttonBlueHover',
            }}
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
    </VStack>
  )
}
