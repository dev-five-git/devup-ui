import { Box } from '@devup-ui/react'
import { Metadata } from 'next'

import { Container } from '@/components/Container'
import { Discord } from '@/components/Discord'

import { Bench } from './Bench'
import { Feature } from './Feature'
import { TopBanner } from './TopBanner'

export const metadata: Metadata = {
  alternates: {
    canonical: '/',
  },
}

export default function HomePage() {
  return (
    <>
      <TopBanner />
      <Bench />
      <Feature />
      <Container>
        <Box px={4} py="40px">
          <Discord />
        </Box>
      </Container>
    </>
  )
}
