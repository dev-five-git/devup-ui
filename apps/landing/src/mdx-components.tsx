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
}

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    ..._components,
  }
}
