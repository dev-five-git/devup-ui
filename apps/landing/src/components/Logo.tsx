import { Center, Flex, Image } from '@devup-ui/react'

import { LogoText } from './LogoText'

export function Logo() {
  return (
    <Flex alignItems="center" gap={[2, null, 3]} h={[7, '42px']}>
      <Image h="100%" src="/logo.svg" />
      <Center h={['18.67px', null, '28px']}>
        <LogoText />
      </Center>
    </Flex>
  )
}
