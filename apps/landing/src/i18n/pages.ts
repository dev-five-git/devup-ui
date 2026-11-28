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
  '/docs/api/box': {
    load: () => import('@/app/(detail)/docs/api/box/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/button': {
    load: () => import('@/app/(detail)/docs/api/button/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/center': {
    load: () => import('@/app/(detail)/docs/api/center/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/css': {
    load: () => import('@/app/(detail)/docs/api/css/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/custom-shorthands': {
    load: () => import('@/app/(detail)/docs/api/custom-shorthands/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/flex': {
    load: () => import('@/app/(detail)/docs/api/flex/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/grid': {
    load: () => import('@/app/(detail)/docs/api/grid/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/group-selector': {
    load: () => import('@/app/(detail)/docs/api/group-selector/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/image': {
    load: () => import('@/app/(detail)/docs/api/image/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/input': {
    load: () => import('@/app/(detail)/docs/api/input/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/selector': {
    load: () => import('@/app/(detail)/docs/api/selector/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/style-props': {
    load: () => import('@/app/(detail)/docs/api/style-props/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/text': {
    load: () => import('@/app/(detail)/docs/api/text/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/api/v-stack': {
    load: () => import('@/app/(detail)/docs/api/v-stack/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/build-errors': {
    load: () => import('@/app/(detail)/docs/build-errors/page.ko.mdx'),
    Layout: DocsLayout,
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
