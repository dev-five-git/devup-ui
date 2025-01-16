import 'sanitize.css'

import type { Metadata } from 'next'

import { Footer } from '../components/Footer'

export const metadata: Metadata = {
  title: 'Devup UI',
  description: 'Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor',
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <head>
        <base
          href={process.env.NODE_ENV === 'production' ? '/devup-ui' : '/'}
        />
      </head>
      <body>
        {children}
        <Footer />
      </body>
    </html>
  )
}
