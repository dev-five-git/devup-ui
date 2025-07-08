import { Box, Text } from '@devup-ui/react'
import type { MDXComponents } from 'mdx/types'

import { Code } from './components/Code'

const _components = {
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
      <Text as="p" color="$text" typography="bodyReg">
        {children}
      </Text>
    )
  },
  pre({ children }: { children: React.ReactNode }) {
    return <Box as="pre">{children}</Box>
  },
}

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    ..._components,
  }
}
