import { Box, css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'

import { OpenMenuItem } from './OpenMenuItem'

export interface MenuItemProps {
  selected?: boolean
  children?: React.ReactNode
  to?: string
  subMenu?: {
    selected?: boolean
    children?: React.ReactNode
    to?: string
  }[]
}

export function MenuItem(props: MenuItemProps) {
  const { selected, children, to, subMenu } = props
  if (subMenu) return <OpenMenuItem {...props} subMenu={subMenu} />
  const inner = (
    <Flex
      alignItems="center"
      bg={selected ? '$menuActive' : undefined}
      borderRadius="6px"
      gap="10px"
      p="6px 10px"
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
      {subMenu && <Image boxSize="16px" src="/menu-arrow.svg" />}
    </Flex>
  )
  return to ? (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href={to}
    >
      {inner}
    </Link>
  ) : (
    inner
  )
}
