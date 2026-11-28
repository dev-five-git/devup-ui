import type { Metadata } from 'next'
import type { ComponentType, ReactNode } from 'react'

import ComponentsLayout from '@/app/(detail)/components/layout'
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
  // Component references share the components layout.
  '/components/overview': {
    load: () => import('@/app/(detail)/components/overview/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/bottom-sheet': {
    load: () =>
      import('@/app/(detail)/components/[component]/bottom-sheet/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/button': {
    load: () => import('@/app/(detail)/components/[component]/button/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/checkbox': {
    load: () =>
      import('@/app/(detail)/components/[component]/checkbox/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/color-picker': {
    load: () =>
      import('@/app/(detail)/components/[component]/color-picker/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/confirm': {
    load: () => import('@/app/(detail)/components/[component]/confirm/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/date-picker': {
    load: () =>
      import('@/app/(detail)/components/[component]/date-picker/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/dropdown': {
    load: () =>
      import('@/app/(detail)/components/[component]/dropdown/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/footer': {
    load: () => import('@/app/(detail)/components/[component]/footer/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/header': {
    load: () => import('@/app/(detail)/components/[component]/header/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/label': {
    load: () => import('@/app/(detail)/components/[component]/label/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/menu': {
    load: () => import('@/app/(detail)/components/[component]/menu/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/pagination': {
    load: () =>
      import('@/app/(detail)/components/[component]/pagination/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/progress-bar': {
    load: () =>
      import('@/app/(detail)/components/[component]/progress-bar/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/radio': {
    load: () => import('@/app/(detail)/components/[component]/radio/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/search': {
    load: () => import('@/app/(detail)/components/[component]/search/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/select': {
    load: () => import('@/app/(detail)/components/[component]/select/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/slider': {
    load: () => import('@/app/(detail)/components/[component]/slider/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/snackbar': {
    load: () =>
      import('@/app/(detail)/components/[component]/snackbar/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/stepper': {
    load: () => import('@/app/(detail)/components/[component]/stepper/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/tab': {
    load: () => import('@/app/(detail)/components/[component]/tab/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/textarea': {
    load: () =>
      import('@/app/(detail)/components/[component]/textarea/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/textbox': {
    load: () => import('@/app/(detail)/components/[component]/textbox/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/theme-button': {
    load: () =>
      import('@/app/(detail)/components/[component]/theme-button/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/toggle': {
    load: () => import('@/app/(detail)/components/[component]/toggle/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/tooltip': {
    load: () => import('@/app/(detail)/components/[component]/tooltip/page.ko'),
    Layout: ComponentsLayout,
  },
  '/components/uploader': {
    load: () =>
      import('@/app/(detail)/components/[component]/uploader/page.ko'),
    Layout: ComponentsLayout,
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
