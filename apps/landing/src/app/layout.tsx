import './markdown.css'
import 'sanitize.css'

import { css, ThemeScript } from '@devup-ui/react'
import type { Metadata } from 'next'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'

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
    <html lang="en" suppressHydrationWarning>
      <head>
        <ThemeScript auto />
        <meta content="width=device-width, initial-scale=1.0" name="viewport" />
        <base
          href={process.env.NODE_ENV === 'production' ? '/devup-ui' : '/'}
        />
      </head>
      <body
        className={css({
          bg: '$containerBackground',
          color: '$text',
        })}
      >
        <Header />
        {children}
        <Footer />
      </body>
    </html>
  )
}
