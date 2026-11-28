import { Box, css, globalCss, ThemeScript } from '@devup-ui/react'
import { resetCss } from '@devup-ui/reset-css'
import ReactLenis from 'lenis/react'
import { type Locale, NextIntlClientProvider } from 'next-intl'
import { getMessages, setRequestLocale } from 'next-intl/server'

import { AnchorScroll } from '@/components/AnchorScroll'
import { Footer } from '@/components/Footer'
import { Header } from '@/components/Header'
import { SearchModal } from '@/components/SearchModal'
import { disableLinkAutoPrefetch } from '@/utils/disable-link-auto-prefetch'

function escapeScriptClosingTag(script: string) {
  return script.replace(/<\/script/giu, '\\u003c/script')
}

const linkAutoPrefetchPolicy = escapeScriptClosingTag(
  `(${disableLinkAutoPrefetch.toString()})()`,
)

// vinext's static export already maps every RSC navigation to the `.txt`
// payload it writes, so the static host needs no fetch transport. Only
// vinext's automatic link prefetching is held back; the CI-only Next build
// has no vinext runtime to configure.
const needsLinkAutoPrefetchPolicy =
  process.env.NODE_ENV === 'production' &&
  process.env.LANDING_BUILD_MODE !== 'next'

const googleTagManagerScript = escapeScriptClosingTag(`(function(w,d,s,l,i){
if(w.location.hostname!=='devup-ui.com')return;
w[l]=w[l]||[];w[l].push({'gtm.start':new Date().getTime(),event:'gtm.js'});
var f=d.getElementsByTagName(s)[0],j=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';
j.async=true;j.src='https://www.googletagmanager.com/gtm.js?id='+i+dl;
f.parentNode.insertBefore(j,f);
})(window,document,'script','dataLayer','GTM-PSRKC4QZ')`)

export const metadata = {
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
}

resetCss()

globalCss({
  pre: {
    borderRadius: '10px',
  },
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
  code: {
    fontFamily: 'D2Coding',
    fontSize: ['13px', '15px'],
    fontStyle: 'normal',
    fontWeight: 700,
    lineHeight: '1.5',
    letterSpacing: '-0.03em',
  },
})

export async function SiteLayout({
  children,
  locale = 'en',
}: Readonly<{
  children: React.ReactNode
  locale?: Locale
}>) {
  setRequestLocale(locale)
  const messages = await getMessages({ locale })
  return (
    <html lang={locale} suppressHydrationWarning>
      <head>
        {needsLinkAutoPrefetchPolicy && (
          <script data-vinext-link-prefetch-policy="">
            {linkAutoPrefetchPolicy}
          </script>
        )}
        <script>{googleTagManagerScript}</script>
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
          <span data-javascript-disabled="" hidden />
        </noscript>
        <NextIntlClientProvider locale={locale} messages={messages}>
          <ReactLenis options={{ duration: 1.4, allowNestedScroll: true }} root>
            <AnchorScroll />
            <SearchModal />
            <Box bg="$background">
              <Header locale={locale} />
              <Box as="main">{children}</Box>
            </Box>
            <Footer />
          </ReactLenis>
        </NextIntlClientProvider>
      </body>
    </html>
  )
}
