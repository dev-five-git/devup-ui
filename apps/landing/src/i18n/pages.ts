import type { Metadata } from 'next'
import type { ComponentType, ReactNode } from 'react'

import DocsLayout from '@/app/(detail)/docs/layout'

interface TranslatedPage {
  load: () => Promise<{ default: ComponentType; metadata?: Metadata }>
  Layout?: ComponentType<{ children: ReactNode }>
}

// Keep translated content beside its English source; share the original layout.
export const translatedPages: Record<string, TranslatedPage | undefined> = {
  '/docs/overview': {
    load: () => import('@/app/(detail)/docs/overview/page.ko.mdx'),
    Layout: DocsLayout,
  },
  '/docs/quick-start': {
    load: () => import('@/app/(detail)/docs/quick-start/page.ko.mdx'),
    Layout: DocsLayout,
  },
}
