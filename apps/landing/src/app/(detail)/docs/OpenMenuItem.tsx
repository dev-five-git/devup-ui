'use client'
import { Box, css, Flex, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Fragment, useReducer } from 'react'

import { URL_PREFIX } from '../../../constants'
import { MenuItemProps } from './MenuItem'

export function OpenMenuItem({
  children,
  subMenu,
}: Omit<MenuItemProps, 'subMenu' | 'to'> &
  Required<Pick<MenuItemProps, 'subMenu'>>) {
  const path = usePathname()
  const selected = subMenu.some((item) =>
    item.to ? path.startsWith(item.to) : false,
  )
  const [open, handleOpen] = useReducer((state) => !state, selected)
  return (
    <>
      <Flex
        _hover={{
          bg: '$menuHover',
        }}
        alignItems="center"
        bg={selected ? '$menuActive' : undefined}
        borderRadius="6px"
        cursor="pointer"
        gap="10px"
        onClick={handleOpen}
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
        {subMenu && (
          <Image
            boxSize="16px"
            src={URL_PREFIX + '/menu-arrow.svg'}
            transform={open ? 'rotate(0deg)' : 'rotate(-90deg)'}
            transition="transform 0.2s"
          />
        )}
      </Flex>
      {open && (
        <Flex gap="8px">
          <Box borderRight="1px solid var(--border, #E0E0E0)" w="10px" />
          <VStack flex="1" gap="4px">
            {subMenu.map(({ children, to }, idx) => {
              const selected = to ? path.startsWith(to) : false
              const inner = (
                <Flex
                  _hover={{
                    bg: '$menuHover',
                  }}
                  alignItems="center"
                  bg={selected ? '$menuActive' : undefined}
                  borderRadius="6px"
                  gap="10px"
                  p="10px"
                >
                  {selected && (
                    <Box bg="$primary" borderRadius="100%" boxSize="8px" />
                  )}
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
                  key={idx}
                  className={css({
                    textDecoration: 'none',
                  })}
                  href={to}
                >
                  {inner}
                </Link>
              ) : (
                <Fragment key={idx}>{inner}</Fragment>
              )
            })}
          </VStack>
        </Flex>
      )}
    </>
  )
}
