import type { Metadata } from 'next'

export const siteMetadata = {
  title: 'Devup UI',
  description: 'Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor',
  alternates: { canonical: 'https://devup-ui.com' },
  metadataBase: new URL('https://devup-ui.com'),
  openGraph: {
    title: 'Devup UI',
    description: 'Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor',
    images: ['https://devup-ui.com/og-image.png'],
    siteName: 'Devup UI',
    type: 'website',
    url: 'https://devup-ui.com',
  },
} satisfies Metadata
