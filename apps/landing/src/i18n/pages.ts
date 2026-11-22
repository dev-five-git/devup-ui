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
  '/docs/devup/breakpoints': {
    load: () => import('@/app/(detail)/docs/devup/breakpoints/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/devup/colors': {
    load: () => import('@/app/(detail)/docs/devup/colors/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/devup/devup-json': {
    load: () => import('@/app/(detail)/docs/devup/devup-json/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/devup/figma-plugin': {
    load: () => import('@/app/(detail)/docs/devup/figma-plugin/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/devup/length': {
    load: () => import('@/app/(detail)/docs/devup/length/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/devup/shadow': {
    load: () => import('@/app/(detail)/docs/devup/shadow/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/devup/typography': {
    load: () => import('@/app/(detail)/docs/devup/typography/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/figma-and-theme-integration/devup-figma-plugin': {
    load: () =>
      import('@/app/(detail)/docs/figma-and-theme-integration/devup-figma-plugin/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/figma-and-theme-integration/devup-json': {
    load: () =>
      import('@/app/(detail)/docs/figma-and-theme-integration/devup-json/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/installation': {
    load: () => import('@/app/(detail)/docs/installation/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/migration/overview': {
    load: () => import('@/app/(detail)/docs/migration/overview/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/migration/styled-components': {
    load: () =>
      import('@/app/(detail)/docs/migration/styled-components/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/migration/stylex': {
    load: () => import('@/app/(detail)/docs/migration/stylex/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/migration/tailwind': {
    load: () => import('@/app/(detail)/docs/migration/tailwind/page.ko.mdx'),
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
