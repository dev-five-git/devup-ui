'use client'
import { Box, css, Flex, Text } from '@devup-ui/react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'

import { URL_PREFIX } from '../../../constants'
import { OpenMenuItem } from './OpenMenuItem'

export interface MenuItemProps {
  children?: React.ReactNode
  to?: string
  subMenu?: {
    children?: React.ReactNode
    to?: string
  }[]
}

export function MenuItem(props: MenuItemProps) {
  const { children, to, subMenu } = props
  const path = usePathname()
  const selected = to
    ? path.startsWith(to) || path.startsWith(URL_PREFIX + to)
    : !!subMenu?.some((item) =>
        item.to
          ? path.startsWith(URL_PREFIX + item.to) || path.startsWith(item.to)
          : false,
      )

  if (subMenu) return <OpenMenuItem {...props} subMenu={subMenu} />
  const inner = (
    <Flex
      _hover={
        selected
          ? undefined
          : {
              bg: '$menuHover',
            }
      }
      _active={{
        bg: '$menuActive',
      }}
      alignItems="center"
      bg={selected ? '$menuActive' : undefined}
      borderRadius="6px"
      gap="10px"
      p={['10px', null, '6px 10px']}
    >
      {selected && <Box bg="$primary" borderRadius="100%" boxSize="8px" />}
      <Text
        color={selected ? '$title' : '$text'}
        flex="1"
        opacity={selected ? '1' : '0.8'}
        typography={selected ? 'buttonS' : 'buttonSmid'}
      >
        {children}
      </Text>
    </Flex>
  )
  return to ? (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href={URL_PREFIX + to}
    >
      {inner}
    </Link>
  ) : (
    inner
  )
}
