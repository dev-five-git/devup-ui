import { Box } from '@devup-ui/react'
import type { Metadata } from 'next'

import { Container } from '@/components/Container'
import { Discord } from '@/components/Discord'
import { Bench } from '@/app/Bench'
import { Feature } from '@/app/Feature'
import { TopBanner } from '@/app/TopBanner'

const description =
  'Zero Config, Zero FOUC, Zero Runtime을 실현하는 CSS-in-JS 전처리기'

export const metadata: Metadata = {
  description,
  alternates: {
    canonical: '/ko',
    languages: {
      en: '/',
      ko: '/ko',
      'x-default': '/',
    },
  },
  openGraph: {
    title: 'Devup UI',
    description,
    images: ['https://devup-ui.com/og-image.png'],
    locale: 'ko_KR',
    siteName: 'Devup UI',
    type: 'website',
    url: '/ko',
  },
}

export default function HomePage() {
  return (
    <>
      <TopBanner locale="ko" />
      <Bench locale="ko" />
      <Feature locale="ko" />
      <Container>
        <Box px={4} py="40px">
          <Discord locale="ko" />
        </Box>
      </Container>
    </>
  )
}
