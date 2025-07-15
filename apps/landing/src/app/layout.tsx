import './markdown.css'
import 'sanitize.css'

import { css, globalCss, ThemeScript } from '@devup-ui/react'
import type { Metadata } from 'next'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { SearchModal } from '../components/SearchModal'

export const metadata: Metadata = {
  title: 'Devup UI',
  description: 'Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor',
  alternates: {
    canonical: 'https://devup-ui.com',
  },
  metadataBase: new URL('https://devup-ui.com'),
  openGraph: {
    title: 'Devup UI',
    description: 'Zero Config, Zero FOUC, Zero Runtime, CSS in JS Preprocessor',
    images: ['https://devup-ui.com/og-image.png'],
    siteName: 'Devup UI',
    type: 'website',
    url: 'https://devup-ui.com',
  },
}

// table {
//     border-collapse: collapse;
//     border-spacing: 0;
//     border: 1px solid var(--text);
//     color: var(--text, #2F2F2F);

//     /* Desktop/bodyReg */
//     font-family: Pretendard;
//     font-size: 16px;
//     font-style: normal;
//     font-weight: 400;
//     line-height: 150%; /* 24px */
//     letter-spacing: -0.48px;
// }

// code {
//     font-family: D2Coding;
//     font-size: 13px;
//     font-style: normal;
//     font-weight: 700;
//     line-height: 1.5;
//     letter-spacing: -0.03em;
// }

// @media (min-width: 1280px) {
//     code {
//         font-size: 15px;
//     }
// }

// th, td {
//     border: 1px solid var(--text);
//     padding: 6px 13px;
// }

// pre {
//     border-radius: 10px;
// }

// console.log('wtf')

// globalCss`
//   body {
//   }
// `
// globalCss({
//   body: {
//     // bg: 'red !important',
//   },
// })

globalCss({
  imports: ['http://cdn.jsdelivr.net/gh/joungkyun/font-d2coding/d2coding.css'],
  table: {
    borderCollapse: 'collapse',
    borderSpacing: 0,
    border: '1px solid var(--text)',
    color: 'var(--text, #2F2F2F)',
    fontFamily: 'Pretendard',
    fontSize: '16px',
    fontStyle: 'normal',
    fontWeight: 400,
    lineHeight: '150%',
    letterSpacing: '-0.48px',
  },
  code: {
    fontFamily: 'D2Coding 궁서',
    fontSize: ['13px', '15px'],
    fontStyle: 'normal',
    fontWeight: 700,
    lineHeight: '1.5',
    letterSpacing: '-0.03em',
  },
  'th, td': {
    border: '1px solid var(--text)',
    padding: '6px 13px',
  },
  pre: {
    borderRadius: '10px',
  },
})

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
