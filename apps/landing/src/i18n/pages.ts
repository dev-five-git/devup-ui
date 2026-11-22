import type { Metadata } from 'next'
import type { ComponentType, ReactNode } from 'react'

import DocsLayout from '@/app/(detail)/docs/layout'
import TeamLayout from '@/app/(detail)/team/layout'

interface TranslatedPage {
  load: () => Promise<{ default: ComponentType; metadata?: Metadata }>
  Layout?: ComponentType<{ children: ReactNode }>
}

// Keep translated content beside its English source; share the original layout.
export const translatedPages: Record<string, TranslatedPage | undefined> = {
  // Home and showcase render inside the shared site layout alone.
  '/': {
    load: () => import('@/app/(detail)/page.ko'),
  },
  '/showcase': {
    load: () => import('@/app/(detail)/showcase/page.ko'),
  },
  '/docs/devup/figma-plugin': {
    load: () => import('@/app/(detail)/docs/devup/figma-plugin/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/installation': {
    load: () => import('@/app/(detail)/docs/installation/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/migration/styled-components': {
    load: () =>
      import('@/app/(detail)/docs/migration/styled-components/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/migration/vanilla-extract': {
    load: () =>
      import('@/app/(detail)/docs/migration/vanilla-extract/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/overview': {
    load: () => import('@/app/(detail)/docs/overview/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/quick-start': {
    load: () => import('@/app/(detail)/docs/quick-start/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/team': {
    load: () => import('@/app/(detail)/team/page.ko.mdx'),
    Layout: TeamLayout,
  },
}
