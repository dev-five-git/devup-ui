import { Box, css, globalCss, ThemeScript } from '@devup-ui/react'
import { resetCss } from '@devup-ui/reset-css'
import ReactLenis from 'lenis/react'
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

resetCss()

globalCss({
  fontFaces: [
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-ExtraBold.woff2) format("woff2")',
      fontWeight: 800,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Bold.woff2) format("woff2")',
      fontWeight: 700,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-SemiBold.woff2) format("woff2")',
      fontWeight: 600,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Medium.woff2) format("woff2")',
      fontWeight: 500,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Regular.woff2) format("woff2")',
      fontWeight: 400,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Light.woff2) format("woff2")',
      fontWeight: 300,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'Pretendard',
      src: 'url(https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Thin.woff2) format("woff2")',
      fontWeight: 100,
      fontStyle: 'normal',
    },
    {
      fontFamily: 'D2Coding',
      src: 'url(https://cdn.jsdelivr.net/gh/projectnoonnu/noonfonts_three@1.0/D2Coding.woff) format("woff")',
      fontWeight: 400,
      fontDisplay: 'swap',
    },
  ],
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
    fontFamily: 'D2Coding',
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
        <link
          as="font"
          crossOrigin="anonymous"
          href="https://cdn.jsdelivr.net/gh/projectnoonnu/noonfonts_three@1.0/D2Coding.woff"
          rel="preload"
          type="font/woff"
        />
        {[
          'ExtraBold',
          'Bold',
          'SemiBold',
          'Medium',
          'Regular',
          'Light',
          'Thin',
        ].map((font) => (
          <link
            key={font}
            as="font"
            crossOrigin="anonymous"
            href={`https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-${font}.woff2`}
            rel="preload"
            type="font/woff2"
          />
        ))}
        <ThemeScript auto />
        <meta content="width=device-width, initial-scale=1.0" name="viewport" />
        <link href="/favicon.ico" rel="shortcut icon" />
      </head>
      <body
        className={css({
          bg: '$footerBg',
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
        <ReactLenis options={{ duration: 1.4 }} root>
          <SearchModal />
          <Box bg="$background">
            <Header />
            {children}
          </Box>
          <Footer />
        </ReactLenis>
      </body>
    </html>
  )
}
