import { Center, Flex, Image } from '@devup-ui/react'

import { URL_PREFIX } from '../constants'
import { LogoText } from './LogoText'

export function Logo() {
  return (
    <Flex alignItems="center" gap={[2, null, 4]} h={[7, '42px']}>
      <Image h="100%" src={URL_PREFIX + '/logo.svg'} />
      <Center h={[5, null, '42px']}>
        <LogoText />
      </Center>
    </Flex>
  )
}
