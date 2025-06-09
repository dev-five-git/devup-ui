import { Box, css, Flex, Text } from '@devup-ui/react'
import Link from 'next/link'

import { LeftMenu } from '../../app/(detail)/docs/LeftMenu'
import { HeaderInput } from './HeaderInput'
import { HeaderInputWrap } from './HeaderInputWrap'
import { MobMenuWrap } from './MobMenuWrap'

export function MobMenu() {
  return (
    <MobMenuWrap>
      <Box px={4} py={2.5}>
        <HeaderInputWrap>
          <HeaderInput readOnly />
        </HeaderInputWrap>
      </Box>
      <Box overflowY="auto" px={4}>
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href="/docs/overview"
        >
          <Flex alignItems="center" py="10px">
            <Text color="$title" textAlign="right" typography="buttonM">
              Docs
            </Text>
          </Flex>
        </Link>
        <LeftMenu />
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href="/team"
        >
          <Flex alignItems="center" py="10px">
            <Text color="$title" textAlign="right" typography="buttonM">
              Team
            </Text>
          </Flex>
        </Link>
      </Box>
    </MobMenuWrap>
  )
}
