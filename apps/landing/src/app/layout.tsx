import './markdown.css'
import 'sanitize.css'

import { css, ThemeScript } from '@devup-ui/react'
import type { Metadata } from 'next'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { SearchModal } from '../components/SearchModal'

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
        <script
          dangerouslySetInnerHTML={{
            __html: `(function(w,d,s,l,i){w[l]=w[l]||[];w[l].push({'gtm.start':
new Date().getTime(),event:'gtm.js'});var f=d.getElementsByTagName(s)[0],
j=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';j.async=true;j.src=
'https://www.googletagmanager.com/gtm.js?id='+i+dl;f.parentNode.insertBefore(j,f);
})(window,document,'script','dataLayer','GTM-PSRKC4QZ')`,
          }}
        />
        <ThemeScript auto />
        <meta content="width=device-width, initial-scale=1.0" name="viewport" />
        <link href="/favicon.ico" rel="shortcut icon" />
      </head>
      <body
        className={css({
          bg: '$background',
          color: '$text',
        })}
      >
        <noscript>
          <iframe
            height="0"
            src="https://www.googletagmanager.com/ns.html?id=GTM-PSRKC4QZ"
            style={{ display: 'none', visibility: 'hidden' }}
            width="0"
          />
        </noscript>
        <SearchModal />
        <Header />
        {children}
        <Footer />
      </body>
    </html>
  )
}
