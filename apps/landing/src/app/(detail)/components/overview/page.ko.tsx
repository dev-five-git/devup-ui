import type { Metadata } from 'next'

import { ComponentsOverview } from './ComponentsOverview'
import Description from './Description.ko.mdx'

export const metadata: Metadata = {
  title: 'Devup UI 컴포넌트',
  description: 'Devup UI에서 사용할 수 있는 컴포넌트 라이브러리를 살펴보세요.',
  alternates: {
    canonical: '/ko/components/overview',
    languages: {
      en: '/components/overview',
      ko: '/ko/components/overview',
      'x-default': '/components/overview',
    },
  },
  openGraph: {
    title: 'Devup UI 컴포넌트',
    description:
      'Devup UI에서 사용할 수 있는 컴포넌트 라이브러리를 살펴보세요.',
    url: '/ko/components/overview',
    siteName: 'Devup UI',
    images: ['/og-image.png'],
    locale: 'ko_KR',
  },
}

export default function Page() {
  return <ComponentsOverview Description={Description} locale="ko" />
}
