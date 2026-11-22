import type { Metadata } from 'next'

import { ComponentsOverview } from './ComponentsOverview'
import Description from './Description.mdx'

export const metadata: Metadata = {
  title: 'Devup UI Components',
  description: 'Explore the component library available in Devup UI.',
  alternates: {
    canonical: '/components/overview',
    languages: {
      en: '/components/overview',
      ko: '/ko/components/overview',
      'x-default': '/components/overview',
    },
  },
  openGraph: {
    title: 'Devup UI Components',
    description: 'Explore the component library available in Devup UI.',
    url: '/components/overview',
    siteName: 'Devup UI',
    images: ['/og-image.png'],
  },
}

export default function Page() {
  return <ComponentsOverview Description={Description} />
}
