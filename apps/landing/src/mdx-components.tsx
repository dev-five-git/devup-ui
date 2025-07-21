import { Box, Text } from '@devup-ui/react'
import type { MDXComponents } from 'mdx/types'

import { Code } from './components/Code'

export const _components = {
  code({ node, inline, className, children, ...props }: any) {
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
      <Text as="h1" color="$title" typography="h1">
        {children}
      </Text>
    )
  },
  h2({ children }: { children: React.ReactNode }) {
    return (
      <Text as="h2" color="$title" typography="h2">
        {children}
      </Text>
    )
  },
  h3({ children }: { children: React.ReactNode }) {
    return (
      <Text as="h3" color="$title" typography="h3">
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
