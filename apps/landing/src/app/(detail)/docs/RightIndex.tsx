'use client'
import { Box, css, Flex, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useEffect, useState } from 'react'

function IndexMenu({
  children,
  selected,
  sub,
  onClick,
}: {
  children: React.ReactNode
  selected?: boolean
  sub?: boolean
  onClick?: () => void
}) {
  return (
    <Flex
      alignItems="center"
      cursor="pointer"
      gap="10px"
      onClick={onClick}
      p={sub ? '4px 10px 4px 30px' : '4px 10px'}
      role="group"
    >
      {selected && <Box bg="$primary" borderRadius="50%" boxSize="6px" />}
      <Text
        _groupHover={{ opacity: '0.8' }}
        color={selected ? '$primary' : '$text'}
        flex="1"
        opacity={selected ? '0.8' : '0.6'}
        typography="caption"
      >
        {children}
      </Text>
    </Flex>
  )
}

export function RightIndex() {
  const pathname = usePathname()
  const editUrl = `https://github.com/dev-five-git/devup-ui/tree/main/apps/landing/src/app/(detail)/components/${pathname.split('components')[1]}/`
  const [menus, setMenus] = useState<
    {
      text: string
      sub?: boolean
      onClick?: () => void
    }[]
  >([])
  useEffect(() => {
    const elements = document.querySelectorAll(
      '.markdown-body h4, .markdown-body h6',
    )
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setMenus(
      [...elements].map((element) => ({
        text: element.textContent!,
        sub: element.tagName === 'H6',
        onClick: () => {
          element.scrollIntoView({ behavior: 'smooth' })
        },
      })),
    )
  }, [pathname])

  return (
    <VStack gap="16px" p="20px 16px" w="200px">
      <VStack>
        <Flex alignItems="center" gap="10px" py="6px">
          <Text color="$text" flex="1" typography="captionBold">
            Contents
          </Text>
        </Flex>
        {menus.map((menu) => (
          <IndexMenu key={menu.text} onClick={menu.onClick} sub={menu.sub}>
            {menu.text}
          </IndexMenu>
        ))}
      </VStack>
      <Box bg="$border" h="1px" />
      <Link
        className={css({
          textDecoration: 'none',
          _hover: {
            textDecoration: 'underline',
            textDecorationColor: '$text',
          },
        })}
        href={editUrl}
        role="group"
        target="_blank"
      >
        <Flex gap="4px">
          <Text
            _groupHover={{ color: '$text' }}
            color="$caption"
            flex="1"
            textAlign="right"
            typography="small"
          >
            Edit this page
          </Text>
          <svg
            className={css({
              color: '$caption',
              _groupHover: { color: '$text' },
            })}
            fill="none"
            height="16"
            viewBox="0 0 16 16"
            width="16"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M13.3344 8.0656H12.1499V5.34691L9.02246 8.48087L8.18399 7.64415L11.3037 4.51788H8.60216V3.33334H13.3344V8.0656Z"
              fill="currentColor"
            />
            <path
              d="M11.49 9.29411V12.8235H3.84297V5.17647H7.37239V4H3.84297C3.19592 4 2.6665 4.52941 2.6665 5.17647V12.8235C2.6665 13.4706 3.19592 14 3.84297 14H11.49C12.1371 14 12.6665 13.4706 12.6665 12.8235V9.29411H11.49Z"
              fill="currentColor"
            />
          </svg>
        </Flex>
      </Link>
    </VStack>
  )
}
