import { Box, Text } from '@devup-ui/react'
import type { MDXComponents } from 'mdx/types'
import { isValidElement, type ReactNode } from 'react'

import { Code } from './components/Code'

interface MarkdownCodeProps extends React.ComponentProps<'code'> {
  inline?: boolean
  node?: unknown
}

function getNodeText(node: ReactNode): string {
  if (typeof node === 'string' || typeof node === 'number') {
    return String(node)
  }
  if (Array.isArray(node)) {
    return node.map(getNodeText).join('')
  }
  if (isValidElement<{ children?: ReactNode }>(node)) {
    return getNodeText(node.props.children)
  }
  return ''
}

function getHeadingId(children: ReactNode): string {
  return getNodeText(children)
    .trim()
    .toLowerCase()
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
}

export const _components = {
  code({ inline, className, children, ...props }: MarkdownCodeProps) {
    const match = /language-(\w+)/.exec(className || '')
    return !inline && match ? (
      <Code
        language={match[1]}
        value={String(children).replace(/\n$/, '')}
        {...props}
      />
    ) : (
      <code className={className} {...props}>
        {children}
      </code>
    )
  },
  h1({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="h1"
        color="$title"
        id={getHeadingId(children)}
        scrollMarginTop="100px"
        typography="h1"
      >
        {children}
      </Text>
    )
  },
  h2({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="h2"
        color="$title"
        id={getHeadingId(children)}
        scrollMarginTop="100px"
        typography="h2"
      >
        {children}
      </Text>
    )
  },
  h3({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="h3"
        color="$title"
        id={getHeadingId(children)}
        scrollMarginTop="100px"
        typography="h3"
      >
        {children}
      </Text>
    )
  },
  p({ children }: { children: React.ReactNode }) {
    return (
      <Text as="p" color="$text" m="0" typography="bodyReg">
        {children}
      </Text>
    )
  },
  pre({ children }: { children: React.ReactNode }) {
    return <Box as="pre">{children}</Box>
  },
  table({ children }: { children: React.ReactNode }) {
    return (
      <Box
        as="table"
        border="none"
        maxW="100%"
        minW="600px"
        selectors={{
          '& thead, & tbody': {
            border: 'none',
          },
        }}
        typography="bodyBold"
      >
        {children}
      </Box>
    )
  },
  thead({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="thead"
        bg="$cardBg"
        border="none"
        color="$captionBold"
        m="0"
        textAlign="left"
        typography="bodyReg"
      >
        {children}
      </Text>
    )
  },
  th({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="th"
        border="none"
        color="$captionBold"
        m="0"
        px="20px"
        py="14px"
      >
        {children}
      </Text>
    )
  },
  tr({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="tr"
        borderBottom="1px solid $border"
        borderTop="1px solid $border"
        color="$text"
        m="0"
        typography="body"
      >
        {children}
      </Text>
    )
  },
  td({ children }: { children: React.ReactNode }) {
    return (
      <Text
        as="td"
        border="none"
        color="$text"
        m="0"
        px="20px"
        py="14px"
        typography="body"
        whiteSpace="pre-wrap"
      >
        {children}
      </Text>
    )
  },
}

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    ..._components,
  }
}
