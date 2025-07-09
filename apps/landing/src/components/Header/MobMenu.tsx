import { Box, css, Flex, Text } from '@devup-ui/react'
import Link from 'next/link'

import { LeftMenu as LeftMenuComponents } from '../../app/(detail)/components/LeftMenu'
import { LeftMenu as LeftMenuDocs } from '../../app/(detail)/docs/LeftMenu'
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
        <LeftMenuDocs />
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href="/components/overview"
        >
          <Flex alignItems="center" py="10px">
            <Text color="$title" textAlign="right" typography="buttonM">
              Components
            </Text>
          </Flex>
        </Link>
        <LeftMenuComponents />
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
        <Link
          className={css({
            textDecoration: 'none',
          })}
          href="/storybook/index.html"
        >
          <Flex alignItems="center" gap="4px" py="10px">
            <Text color="$title" textAlign="right" typography="buttonM">
              Storybook
            </Text>
            <Box
              _groupActive={{
                bg: '$primary',
              }}
              bg="$text"
              boxSize="20px"
              maskImage="url(/outlink.svg)"
              maskPosition="center"
              maskRepeat="no-repeat"
              maskSize="contain"
            />
          </Flex>
        </Link>
        <Box h="100px" />
      </Box>
    </MobMenuWrap>
  )
}
